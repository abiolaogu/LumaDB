package platform

import (
	"github.com/lumadb/cluster/pkg/cluster"
	"github.com/lumadb/cluster/pkg/platform/auth"
	"github.com/lumadb/cluster/pkg/platform/cron"
	"github.com/lumadb/cluster/pkg/platform/federation"
	"github.com/lumadb/cluster/pkg/platform/graphql"
	"github.com/lumadb/cluster/pkg/platform/mcp"
	"go.uber.org/zap"
)

// Platform manages the high-level application features (GraphQL, Events, Auth)
type Platform struct {
	node       *cluster.Node
	logger     *zap.Logger
	mcpServer  *mcp.MCPServer
	gqlEngine  *graphql.GraphQLEngine
	authEngine *auth.AuthEngine
	cron       *cron.Scheduler
	registry   *federation.SourceRegistry
}

func NewPlatform(node *cluster.Node, logger *zap.Logger) *Platform {
	return &Platform{
		node:     node,
		logger:   logger,
		cron:     cron.NewScheduler(node, logger),
		registry: federation.NewSourceRegistry(),
	}
}

// Start initializes all platform subsystems
func (p *Platform) Start() error {
	p.logger.Info("Starting Luma Platform...")

	// 0. Start Cron
	p.cron.Start()
	// Note: p.cron.Stop() should be called on shutdown, but for MVP we rely on process exit

	// 1. Start GraphQL Engine (needed by MCP)
	p.gqlEngine = graphql.NewGraphQLEngine(p.node, p.registry, p.logger)
	if err := p.gqlEngine.BuildSchema(); err != nil {
		p.logger.Error("Failed to build GraphQL schema", zap.Error(err))
		return err
	}

	// 2. Start MCP Server
	p.mcpServer = mcp.NewMCPServer(p.node, p.gqlEngine, p.logger)

	// 3. Start Auth Engine
	var err error
	p.authEngine, err = auth.NewAuthEngine(p.node, p.logger)
	if err != nil {
		p.logger.Error("Failed to initialize Auth Engine", zap.Error(err))
		return err
	}
	if err := p.authEngine.Start(); err != nil {
		p.logger.Error("Failed to start Auth Engine", zap.Error(err))
		return err
	}

	return nil
}
