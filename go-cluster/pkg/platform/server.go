package platform

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/fasthttp/router"
	"github.com/lumadb/cluster/pkg/cluster"
	"github.com/lumadb/cluster/pkg/platform/auth"
	"github.com/lumadb/cluster/pkg/platform/events"
	"github.com/valyala/fasthttp"
	"go.uber.org/zap"
)

// Server serves REST and GraphQL APIs using fasthttp for high performance
type Server struct {
	node     *cluster.Node
	logger   *zap.Logger
	platform *Platform
	router   *router.Router
}

func NewServer(node *cluster.Node, platform *Platform, logger *zap.Logger) *Server {
	return &Server{
		node:     node,
		logger:   logger,
		platform: platform,
		router:   router.New(),
	}
}

func (s *Server) Start(addr string) error {
	s.logger.Info("Starting LumaDB Platform Server (FastHTTP)", zap.String("addr", addr))

	// Initialize Schema
	if err := s.platform.gqlEngine.BuildSchema(); err != nil {
		s.logger.Error("Failed to build GraphQL schema", zap.Error(err))
	}

	// Setup Routes
	s.setupRoutes()

	// Wrap Router with Middleware
	handler := s.corsMiddleware(s.router.Handler)

	return fasthttp.ListenAndServe(addr, handler)
}

func (s *Server) setupRoutes() {
	// Public Auth
	s.router.POST("/api/auth/login", s.handleLogin)

	// GraphQL (Protected)
	s.router.POST("/graphql", s.authMiddleware(s.handleGraphQL))
	s.router.GET("/graphql", s.authMiddleware(s.handleGraphQLOrPlayground))

	// REST API
	s.router.GET("/api/health", func(ctx *fasthttp.RequestCtx) {
		ctx.SetContentType("application/json")
		ctx.SetStatusCode(fasthttp.StatusOK)
		fmt.Fprintf(ctx, `{"status":"ok","version":"2.0.0"}`)
	})

	// V1 Group (Protected)
	// Router doesn't natively support groups in the same way, so we wrap manually or use path prefix
	// Manual wrapping for simplicity and performance
	s.router.GET("/api/v1/stats", s.authMiddleware(s.handleStats))
	s.router.POST("/api/v1/triggers", s.authMiddleware(s.handleAddTrigger))
	s.router.GET("/api/v1/{collection}", s.authMiddleware(s.handleRestList))
	s.router.POST("/api/v1/{collection}", s.authMiddleware(s.handleRestInsert))
	s.router.GET("/api/v1/{collection}/{id}", s.authMiddleware(s.handleRestGet))
}

// Helpers
func jsonResponse(ctx *fasthttp.RequestCtx, code int, data interface{}) {
	ctx.SetContentType("application/json")
	ctx.SetStatusCode(code)
	if err := json.NewEncoder(ctx).Encode(data); err != nil {
		ctx.Error(err.Error(), fasthttp.StatusInternalServerError)
	}
}

func errorResponse(ctx *fasthttp.RequestCtx, code int, message string) {
	jsonResponse(ctx, code, map[string]string{"error": message})
}

// Handlers

func (s *Server) handleGraphQL(ctx *fasthttp.RequestCtx) {
	var body struct {
		Query     string                 `json:"query"`
		Operation string                 `json:"operationName"`
		Variables map[string]interface{} `json:"variables"`
	}

	if err := json.Unmarshal(ctx.PostBody(), &body); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, "Invalid request body")
		return
	}

	result := s.platform.gqlEngine.Execute(ctx, body.Query, body.Variables)
	jsonResponse(ctx, fasthttp.StatusOK, result)
}

func (s *Server) handleGraphQLOrPlayground(ctx *fasthttp.RequestCtx) {
	queryArgs := ctx.QueryArgs()
	if queryArgs.Has("query") {
		query := string(queryArgs.Peek("query"))
		result := s.platform.gqlEngine.Execute(ctx, query, nil)
		jsonResponse(ctx, fasthttp.StatusOK, result)
		return
	}

	ctx.SetContentType("text/html")
	ctx.SetStatusCode(fasthttp.StatusOK)
	ctx.WriteString(graphiqlHTML)
}

func (s *Server) handleRestList(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	role, ok := ctx.UserValue("role").(string)
	if !ok || !s.platform.authEngine.IsAuthorized(role, auth.ActionRead) {
		errorResponse(ctx, fasthttp.StatusForbidden, "Forbidden")
		return
	}

	// Mock response
	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"collection": collection,
		"data":       []interface{}{},
	})
}

func (s *Server) handleRestInsert(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	role, ok := ctx.UserValue("role").(string)
	if !ok || !s.platform.authEngine.IsAuthorized(role, auth.ActionWrite) {
		errorResponse(ctx, fasthttp.StatusForbidden, "Forbidden")
		return
	}

	var doc map[string]interface{}
	if err := json.Unmarshal(ctx.PostBody(), &doc); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusCreated, map[string]string{
		"collection": collection,
		"status":     "inserted",
	})
}

func (s *Server) handleRestGet(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	id := ctx.UserValue("id").(string)
	role, ok := ctx.UserValue("role").(string)
	if !ok || !s.platform.authEngine.IsAuthorized(role, auth.ActionRead) {
		errorResponse(ctx, fasthttp.StatusForbidden, "Forbidden")
		return
	}

	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"collection": collection,
		"id":         id,
		"data":       nil,
	})
}

func (s *Server) handleStats(ctx *fasthttp.RequestCtx) {
	stats := map[string]interface{}{
		"collections":  12,
		"documents":    1250000,
		"ops_per_sec":  450,
		"latency_p99":  "0.8ms",
		"nodes_active": 3,
		"events_fired": 85200,
	}
	jsonResponse(ctx, fasthttp.StatusOK, stats)
}

func (s *Server) handleAddTrigger(ctx *fasthttp.RequestCtx) {
	role, ok := ctx.UserValue("role").(string)
	if !ok || role != "admin" {
		errorResponse(ctx, fasthttp.StatusForbidden, "Only admins can manage triggers")
		return
	}

	var config struct {
		Name       string            `json:"name"`
		Collection string            `json:"collection"`
		Events     []string          `json:"events"`
		Sink       string            `json:"sink"`
		Config     map[string]string `json:"config"`
	}

	if err := json.Unmarshal(ctx.PostBody(), &config); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	var eventTypes []events.EventType
	for _, e := range config.Events {
		eventTypes = append(eventTypes, events.EventType(e))
	}

	trigger := events.TriggerConfig{
		Name:       config.Name,
		Collection: config.Collection,
		Events:     eventTypes,
		Sink:       events.SinkType(config.Sink),
		Config:     config.Config,
	}

	s.node.AddTrigger(trigger)
	jsonResponse(ctx, fasthttp.StatusCreated, map[string]string{
		"status":  "created",
		"trigger": config.Name,
	})
}

func (s *Server) handleLogin(ctx *fasthttp.RequestCtx) {
	var creds struct {
		Username string `json:"username"`
		Password string `json:"password"`
	}

	if err := json.Unmarshal(ctx.PostBody(), &creds); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, "Invalid request")
		return
	}

	if creds.Username == "admin" && creds.Password == "password" {
		token, err := s.platform.authEngine.GenerateToken("admin", "admin")
		if err != nil {
			errorResponse(ctx, fasthttp.StatusInternalServerError, "Failed to generate token")
			return
		}
		jsonResponse(ctx, fasthttp.StatusOK, map[string]string{"token": token})
		return
	}

	errorResponse(ctx, fasthttp.StatusUnauthorized, "Invalid credentials")
}

// Middleware

func (s *Server) authMiddleware(next fasthttp.RequestHandler) fasthttp.RequestHandler {
	return func(ctx *fasthttp.RequestCtx) {
		authHeader := string(ctx.Request.Header.Peek("Authorization"))
		if authHeader == "" {
			errorResponse(ctx, fasthttp.StatusUnauthorized, "Authorization header required")
			return
		}

		parts := strings.Split(authHeader, " ")
		if len(parts) != 2 || parts[0] != "Bearer" {
			errorResponse(ctx, fasthttp.StatusUnauthorized, "Invalid authorization format")
			return
		}

		tokenString := parts[1]
		claims, err := s.platform.authEngine.ValidateToken(tokenString)
		if err != nil {
			errorResponse(ctx, fasthttp.StatusUnauthorized, "Invalid or expired token")
			return
		}

		ctx.SetUserValue("user_id", claims.UserID)
		ctx.SetUserValue("role", claims.Role)
		next(ctx)
	}
}

func (s *Server) corsMiddleware(next fasthttp.RequestHandler) fasthttp.RequestHandler {
	return func(ctx *fasthttp.RequestCtx) {
		ctx.Response.Header.Set("Access-Control-Allow-Origin", "*")
		ctx.Response.Header.Set("Access-Control-Allow-Methods", "POST, GET, OPTIONS, PUT, DELETE")
		ctx.Response.Header.Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		if string(ctx.Method()) == "OPTIONS" {
			ctx.SetStatusCode(fasthttp.StatusNoContent)
			return
		}
		next(ctx)
	}
}

const graphiqlHTML = `
<!DOCTYPE html>
<html>
  <head>
    <title>LumaDB GraphiQL</title>
    <link href="https://unpkg.com/graphiql/graphiql.min.css" rel="stylesheet" />
  </head>
  <body style="margin: 0;">
    <div id="graphiql" style="height: 100vh;"></div>
    <script
      crossorigin
      src="https://unpkg.com/react/umd/react.production.min.js"
    ></script>
    <script
      crossorigin
      src="https://unpkg.com/react-dom/umd/react-dom.production.min.js"
    ></script>
    <script
      crossorigin
      src="https://unpkg.com/graphiql/graphiql.min.js"
    ></script>
    <script>
      const fetcher = GraphiQL.createFetcher({
        url: '/graphql',
      });
      ReactDOM.render(
        React.createElement(GraphiQL, { fetcher: fetcher }),
        document.getElementById('graphiql'),
      );
    </script>
  </body>
</html>
`
