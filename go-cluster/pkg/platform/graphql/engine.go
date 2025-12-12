package graphql

import (
	"context"
	"fmt"

	"github.com/graphql-go/graphql"
	"github.com/graphql-go/graphql/gqlerrors"
	"github.com/graphql-go/graphql/language/ast"
	"github.com/lumadb/cluster/pkg/cluster"
	"github.com/lumadb/cluster/pkg/platform/federation"
	"go.uber.org/zap"
)

// GraphQLEngine manages the dynamic GraphQL schema
type GraphQLEngine struct {
	node      *cluster.Node
	logger    *zap.Logger
	registry  *federation.SourceRegistry
	schema    graphql.Schema
	hasSchema bool
}

func NewGraphQLEngine(node *cluster.Node, registry *federation.SourceRegistry, logger *zap.Logger) *GraphQLEngine {
	return &GraphQLEngine{
		node:     node,
		registry: registry,
		logger:   logger,
	}
}

// BuildSchema dynamically constructs the GraphQL schema from database collections AND federated sources
func (e *GraphQLEngine) BuildSchema() error {
	e.logger.Info("Building GraphQL Schema...")

	// Root Query
	queryFields := graphql.Fields{
		"hello": &graphql.Field{
			Type: graphql.String,
			Resolve: func(p graphql.ResolveParams) (interface{}, error) {
				return "world", nil
			},
		},
	}

	// Root Mutation
	mutationFields := graphql.Fields{
		"noop": &graphql.Field{
			Type: graphql.String,
			Resolve: func(p graphql.ResolveParams) (interface{}, error) {
				return "ok", nil
			},
		},
	}

	// Custom JSON scalar
	jsonScalar := graphql.NewScalar(graphql.ScalarConfig{
		Name:        "JSON",
		Description: "The generic JSON scalar type represents a JSON value.",
		Serialize: func(value interface{}) interface{} {
			return value
		},
		ParseValue: func(value interface{}) interface{} {
			return value
		},
		ParseLiteral: func(valueAST ast.Value) interface{} {
			switch valueAST := valueAST.(type) {
			case *ast.StringValue:
				return valueAST.Value
			default:
				return nil
			}
		},
	})

	// 1. List all collections to build schema dynamically
	collections, err := e.node.ListCollections()
	if err != nil {
		e.logger.Error("Failed to list collections for schema build", zap.Error(err))
	}

	for _, colName := range collections {
		// Define Type for Collection
		objType := graphql.NewObject(graphql.ObjectConfig{
			Name: colName,
			Fields: graphql.Fields{
				"_id":      &graphql.Field{Type: graphql.String},
				"_created": &graphql.Field{Type: graphql.String},
				"data":     &graphql.Field{Type: jsonScalar},
			},
		})

		// --- QUERIES ---
		// 1. Get by ID
		queryFields[colName+"_by_pk"] = &graphql.Field{
			Type: objType,
			Args: graphql.FieldConfigArgument{
				"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.String)},
			},
			Resolve: func(p graphql.ResolveParams) (interface{}, error) {
				id, _ := p.Args["id"].(string)
				return e.node.GetDocument(colName, id)
			},
		}

		// 2. List
		queryFields[colName] = &graphql.Field{
			Type: graphql.NewList(objType),
			Args: graphql.FieldConfigArgument{
				"limit": &graphql.ArgumentConfig{Type: graphql.Int},
				"where": &graphql.ArgumentConfig{Type: jsonScalar},
			},
			Resolve: func(p graphql.ResolveParams) (interface{}, error) {
				limit, _ := p.Args["limit"].(int)
				if limit <= 0 {
					limit = 10
				}
				query := map[string]interface{}{"limit": limit}
				if whereVal, ok := p.Args["where"].(map[string]interface{}); ok {
					query["filter"] = whereVal
				}
				return e.node.RunQuery(colName, query)
			},
		}

		// --- MUTATIONS ---
		// 3. Insert
		mutationFields["insert_"+colName] = &graphql.Field{
			Type: graphql.String, // Returns ID
			Args: graphql.FieldConfigArgument{
				"data": &graphql.ArgumentConfig{Type: graphql.NewNonNull(jsonScalar)},
			},
			Resolve: func(p graphql.ResolveParams) (interface{}, error) {
				data, _ := p.Args["data"].(map[string]interface{})
				return e.node.InsertDocument(colName, data)
			},
		}
	}

	// 2. Stitched Federated Sources - Native Only
	if e.registry != nil {
		sources := e.registry.List()
		for srcName, src := range sources {
			// In native-only mode, we only support LumaDB sources or similar NoSQL
			// SQL introspection logic is removed.
			// Future: Implement LumaDB-to-LumaDB federation here.
			e.logger.Info("Federated source present but SQL stitching disabled", zap.String("source", srcName))

			// Introspect source (Generic)
			schema, err := src.Introspect(context.Background())
			if err != nil {
				e.logger.Error("Failed to introspect source", zap.String("source", srcName), zap.Error(err))
				continue
			}

			// TODO: Implement generic stitching for non-SQL sources if needed
			// For now, we skip SQL table generation
			_ = schema
		}
	}

	// Finalize Schema
	schemaConfig := graphql.SchemaConfig{
		Query:    graphql.NewObject(graphql.ObjectConfig{Name: "Query", Fields: queryFields}),
		Mutation: graphql.NewObject(graphql.ObjectConfig{Name: "Mutation", Fields: mutationFields}),
	}

	schema, err := graphql.NewSchema(schemaConfig)
	if err != nil {
		return fmt.Errorf("failed to create schema: %v", err)
	}

	e.schema = schema
	e.hasSchema = true
	return nil
}

// Execute runs a GraphQL query
func (e *GraphQLEngine) Execute(ctx context.Context, query string, variables map[string]interface{}) *graphql.Result {
	if !e.hasSchema {
		// Try to build schema lazily
		if err := e.BuildSchema(); err != nil {
			return &graphql.Result{Errors: []gqlerrors.FormattedError{{Message: err.Error()}}}
		}
	}

	params := graphql.Params{
		Schema:         e.schema,
		RequestString:  query,
		VariableValues: variables,
		Context:        ctx,
	}

	return graphql.Do(params)
}
