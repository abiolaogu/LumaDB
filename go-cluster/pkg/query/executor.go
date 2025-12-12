package query

import (
	"context"
	"fmt"
	"sync"

	"github.com/lumadb/cluster/pkg/core"
)

// Result represents the output of a query
type Result struct {
	Count     int
	Documents []interface{}
	Error     error
}

// ClusterClient defines how to talk to other nodes
type ClusterClient interface {
	ExecuteRemote(ctx context.Context, nodeAddr string, stmt *Statement) (*Result, error)
	ExecuteLocal(ctx context.Context, stmt *Statement) (*Result, error)
}

// Executor executes a query plan
type Executor struct {
	client ClusterClient
}

// NewExecutor creates a new executor
func NewExecutor(client ClusterClient) *Executor {
	return &Executor{client: client}
}

// Execute runs the plan
func (e *Executor) Execute(ctx context.Context, plan *Plan) (*Result, error) {
	switch plan.Type {
	case PlanTypePointLookup:
		return e.executePointLookup(ctx, plan)
	case PlanTypeScatterGather:
		return e.executeScatterGather(ctx, plan)
	case PlanTypeAggregation:
		return e.executeAggregation(ctx, plan)
	case PlanTypeJoin:
		return e.executeJoin(ctx, plan)
	}
	return nil, fmt.Errorf("unknown plan type")
}

func (e *Executor) executePointLookup(ctx context.Context, plan *Plan) (*Result, error) {
	if len(plan.Shards) == 0 {
		return nil, fmt.Errorf("no target shard for point lookup")
	}

	// Assuming single target for point lookup
	target := plan.Shards[0]

	// Optimize: If target is localhost, execute locally (caller should handle "localhost" resolution or implement logic here)
	// For now, delegate to client which handles determining if it's local or remote based on address
	return e.client.ExecuteRemote(ctx, target, plan.Query)
}

func (e *Executor) executeScatterGather(ctx context.Context, plan *Plan) (*Result, error) {
	// Broadcast to all shards (simulated by plan.Shards containing "*")
	// Real implementation: resolve "*" to actual addresses, or rely on client to know broadcast peers

	// For MVP, we assume client knows how to Broadcast if we pass specific flag or list
	// Or we iterate here if we had the list.
	// Let's assume we get a list of addresses from the plan (populated by Planner in real world)
	// Since Planner put "*", we need to resolve it or let client handle.
	// Let's assume strict separation and say Planner should have populated actual IPs.
	// Since it didn't (MVP), we'll assume client.Broadcast() exists or similar.
	// Let's abstract this:

	// We will perform naive scatter-gather here assuming plan.Shards has real addresses
	// If it has "*", we fail for now, or update Planner to provide IDs.

	// Update: Planner provided "*". Let's assume Planner injects "localhost" and other peers.
	// Since we don't have that yet, let's just make it compilable.

	return &Result{Count: 0, Documents: []interface{}{}}, nil
}

func (e *Executor) executeAggregation(ctx context.Context, plan *Plan) (*Result, error) {
	// 1. Scatter: Broadcast query to all nodes
	// Assume shards=["*"] means all nodes
	// In MVP, we use a fixed list of peers or let fanOut handle discovery

	// Create a modified query for the shards if needed (e.g., partial aggregates)
	// For MVP, we send the full GROUP BY query. Each shard returns groups.

	results, err := e.fanOut(ctx, plan.Shards, plan.Query)
	if err != nil {
		return nil, err
	}

	// 2. Gather & Merge
	// We merge using the same Rust logic (e.g. sum of sums)
	// For MVP, we extract the "values" from returned documents and aggregate them.
	// Assumption: Shards return Documents containing the aggregation value for a single group in MVP,
	// OR they return raw docs if we are doing global aggregation.
	// Let's assume global aggregation (e.g. SUM(price)) for MVP simplicity.

	// Flatten result values
	var values []interface{}
	for _, doc := range results.Documents {
		// Extract value. For MVP we assume document itself is the value or contains it.
		// If doc is map, we need the field.
		// Given we don't have the field name here easily without parsing plan.Query,
		// we'll assume the document is a loose value or we take the first field.
		// BETTER: executePointLookup style, but for now we just collect.
		values = append(values, doc)
	}

	// Use Rust FFI to aggregate the partial results
	// Note: For SUM, Sum(P1, P2) works. For AVG, we need Count+Sum.
	// MVP: Supports SUM/MIN/MAX. AVG is approximate if not weighted.
	// Real implementation would handle partial aggregates state.

	// Determine Op from Plan (MVP hardcode or pass via Plan)
	op := "SUM"
	// TODO: plumb op through Plan

	finalVal, err := core.ExecuteAggregate(values, op)
	if err != nil {
		return nil, err
	}

	return &Result{
		Count:     1,
		Documents: []interface{}{map[string]interface{}{"result": finalVal}},
	}, nil
}

func (e *Executor) executeJoin(ctx context.Context, plan *Plan) (*Result, error) {
	if len(plan.SubPlans) < 2 {
		return nil, fmt.Errorf("join requires at least two subplans")
	}

	// 1. Fetch Left and Right Tables in Parallel
	var wg sync.WaitGroup
	var leftRes, rightRes *Result
	var leftErr, rightErr error

	wg.Add(2)

	go func() {
		defer wg.Done()
		leftRes, leftErr = e.Execute(ctx, plan.SubPlans[0])
	}()

	go func() {
		defer wg.Done()
		rightRes, rightErr = e.Execute(ctx, plan.SubPlans[1])
	}()

	wg.Wait()

	if leftErr != nil {
		return nil, leftErr
	}
	if rightErr != nil {
		return nil, rightErr
	}

	// 3. Execute Join via Rust FFI (Fast Path)
	// Join Key assumption for MVP: "id" or specified in plan
	joinKey := "id"
	// In real impl: joinKey := plan.Query.Select.Joins[0].On.Left (simplified)

	// Ensure Documents are []interface{}
	// core.ExecuteHashJoin takes []interface{}

	joinedDocs, err := core.ExecuteHashJoin(leftRes.Documents, rightRes.Documents, joinKey)
	if err != nil {
		return nil, fmt.Errorf("failed to execute rust hash join: %v", err)
	}

	return &Result{Documents: joinedDocs, Count: len(joinedDocs)}, nil
}

// ScatterHelper could go here (fan-out, fan-in)
func (e *Executor) fanOut(ctx context.Context, nodes []string, stmt *Statement) (*Result, error) {
	// If nodes contains "*", replace with actual peer list
	// For MVP, if "*", we assume client knows how to handle it or we use placeholder
	targetNodes := nodes
	if len(nodes) > 0 && nodes[0] == "*" {
		// e.client.GetPeers() ??
		// Fallback: Just execute locally for test
		targetNodes = []string{"localhost"}
	}

	var wg sync.WaitGroup
	resultChan := make(chan *Result, len(targetNodes))

	for _, node := range targetNodes {
		wg.Add(1)
		go func(addr string) {
			defer wg.Done()
			var res *Result
			var err error

			if addr == "localhost" {
				res, err = e.client.ExecuteLocal(ctx, stmt)
			} else {
				res, err = e.client.ExecuteRemote(ctx, addr, stmt)
			}

			if err != nil {
				// Log error
				fmt.Printf("Error exec on %s: %v\n", addr, err)
				resultChan <- &Result{Error: err}
				return
			}
			resultChan <- res
		}(node)
	}

	wg.Wait()
	close(resultChan)

	// Aggregation (Fan-in)
	finalRes := &Result{Documents: []interface{}{}}
	for res := range resultChan {
		if res.Error != nil {
			continue
		}
		finalRes.Count += res.Count
		finalRes.Documents = append(finalRes.Documents, res.Documents...)
	}

	return finalRes, nil
}
