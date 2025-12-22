package dialects

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"time"
)

// InfluxQLHandler handles InfluxQL queries
type InfluxQLHandler struct{}

func (h *InfluxQLHandler) Dialect() Dialect {
	return DialectInfluxQL
}

func (h *InfluxQLHandler) Parse(query string) (*ParsedQuery, error) {
	query = strings.TrimSpace(query)
	upper := strings.ToUpper(query)

	parsed := &ParsedQuery{
		Dialect:       DialectInfluxQL,
		OriginalQuery: query,
	}

	// Parse FROM clause
	fromRe := regexp.MustCompile(`(?i)FROM\s+["']?(\w+)["']?`)
	if matches := fromRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Sources = append(parsed.Sources, DataSource{
			Name: matches[1],
		})
	}

	// Parse time range
	timeRe := regexp.MustCompile(`(?i)WHERE\s+.*time\s*([><]=?)\s*now\(\)\s*-\s*(\d+)([smhd])`)
	if matches := timeRe.FindStringSubmatch(query); len(matches) > 3 {
		value, _ := strconv.ParseInt(matches[2], 10, 64)
		unit := matches[3]

		var duration time.Duration
		switch unit {
		case "s":
			duration = time.Duration(value) * time.Second
		case "m":
			duration = time.Duration(value) * time.Minute
		case "h":
			duration = time.Duration(value) * time.Hour
		case "d":
			duration = time.Duration(value) * 24 * time.Hour
		}

		parsed.TimeRange = &TimeRange{
			End:      time.Now(),
			Start:    time.Now().Add(-duration),
			Duration: duration,
		}
	}

	// Parse aggregations
	aggRe := regexp.MustCompile(`(?i)(mean|sum|count|min|max|first|last|median|stddev|spread)\s*\(\s*["']?(\w+)["']?\s*\)`)
	for _, match := range aggRe.FindAllStringSubmatch(query, -1) {
		if len(match) > 2 {
			parsed.Aggregations = append(parsed.Aggregations, Aggregation{
				Function: strings.ToLower(match[1]),
				Column:   match[2],
			})
		}
	}

	// Parse GROUP BY time
	groupTimeRe := regexp.MustCompile(`(?i)GROUP\s+BY\s+time\s*\(\s*(\d+)([smhd])\s*\)`)
	if matches := groupTimeRe.FindStringSubmatch(query); len(matches) > 2 {
		parsed.GroupBy = append(parsed.GroupBy, fmt.Sprintf("time(%s%s)", matches[1], matches[2]))
	}

	// Parse LIMIT
	limitRe := regexp.MustCompile(`(?i)LIMIT\s+(\d+)`)
	if matches := limitRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Limit, _ = strconv.ParseInt(matches[1], 10, 64)
	}

	// Parse ORDER BY
	orderRe := regexp.MustCompile(`(?i)ORDER\s+BY\s+(\w+)(?:\s+(ASC|DESC))?`)
	if matches := orderRe.FindStringSubmatch(query); len(matches) > 1 {
		ascending := true
		if len(matches) > 2 && strings.ToUpper(matches[2]) == "DESC" {
			ascending = false
		}
		parsed.OrderBy = append(parsed.OrderBy, OrderBy{
			Column:    matches[1],
			Ascending: ascending,
		})
	}

	_ = upper // silence unused variable
	return parsed, nil
}

func (h *InfluxQLHandler) FormatResponse(result *QueryResult, format string) (interface{}, error) {
	// Format as InfluxDB response
	type series struct {
		Name    string          `json:"name"`
		Columns []string        `json:"columns"`
		Values  [][]interface{} `json:"values"`
	}

	type resultType struct {
		StatementID int      `json:"statement_id"`
		Series      []series `json:"series"`
	}

	s := series{
		Columns: make([]string, len(result.Columns)),
		Values:  result.Rows,
	}

	for i, col := range result.Columns {
		s.Columns[i] = col.Name
	}

	if len(result.Columns) > 0 {
		// Try to get measurement name from first column
		s.Name = "results"
	}

	return map[string]interface{}{
		"results": []resultType{
			{
				StatementID: 0,
				Series:      []series{s},
			},
		},
	}, nil
}

// FluxHandler handles Flux queries
type FluxHandler struct{}

func (h *FluxHandler) Dialect() Dialect {
	return DialectFlux
}

func (h *FluxHandler) Parse(query string) (*ParsedQuery, error) {
	query = strings.TrimSpace(query)

	parsed := &ParsedQuery{
		Dialect:       DialectFlux,
		OriginalQuery: query,
	}

	// Parse bucket
	bucketRe := regexp.MustCompile(`from\s*\(\s*bucket\s*:\s*"([^"]+)"\s*\)`)
	if matches := bucketRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Database = matches[1]
		parsed.Sources = append(parsed.Sources, DataSource{
			Name:     matches[1],
			Database: matches[1],
		})
	}

	// Parse range
	rangeRe := regexp.MustCompile(`\|>\s*range\s*\(\s*start\s*:\s*(-?\d+[smhd])`)
	if matches := rangeRe.FindStringSubmatch(query); len(matches) > 1 {
		duration := parseFluxDuration(matches[1])
		if duration > 0 {
			parsed.TimeRange = &TimeRange{
				End:      time.Now(),
				Start:    time.Now().Add(-duration),
				Duration: duration,
			}
		}
	}

	// Parse measurement filter
	measurementRe := regexp.MustCompile(`r\._measurement\s*==\s*"([^"]+)"`)
	if matches := measurementRe.FindStringSubmatch(query); len(matches) > 1 {
		if len(parsed.Sources) == 0 {
			parsed.Sources = append(parsed.Sources, DataSource{Name: matches[1]})
		}
	}

	// Parse aggregateWindow
	aggWindowRe := regexp.MustCompile(`\|>\s*aggregateWindow\s*\(\s*every\s*:\s*(\d+[smhd])\s*,\s*fn\s*:\s*(\w+)`)
	if matches := aggWindowRe.FindStringSubmatch(query); len(matches) > 2 {
		parsed.GroupBy = append(parsed.GroupBy, matches[1])
		parsed.Aggregations = append(parsed.Aggregations, Aggregation{
			Function: matches[2],
			Column:   "_value",
		})
	}

	// Parse limit
	limitRe := regexp.MustCompile(`\|>\s*limit\s*\(\s*n\s*:\s*(\d+)\s*\)`)
	if matches := limitRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Limit, _ = strconv.ParseInt(matches[1], 10, 64)
	}

	return parsed, nil
}

func (h *FluxHandler) FormatResponse(result *QueryResult, format string) (interface{}, error) {
	if format == "csv" {
		return formatAsCSV(result), nil
	}
	return formatAsJSON(result), nil
}

// PromQLHandler handles PromQL queries
type PromQLHandler struct{}

func (h *PromQLHandler) Dialect() Dialect {
	return DialectPromQL
}

func (h *PromQLHandler) Parse(query string) (*ParsedQuery, error) {
	query = strings.TrimSpace(query)

	parsed := &ParsedQuery{
		Dialect:       DialectPromQL,
		OriginalQuery: query,
	}

	// First, try to extract metric from inside function calls like rate(metric{...}[5m])
	// Look for the innermost metric selector
	innerSelectorRe := regexp.MustCompile(`([a-zA-Z_:][a-zA-Z0-9_:]*)\s*\{([^}]*)\}\s*(?:\[(\d+[smhdwy])\])?`)
	if matches := innerSelectorRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Sources = append(parsed.Sources, DataSource{
			Name: matches[1],
		})

		// Parse labels
		if len(matches) > 2 && matches[2] != "" {
			labelRe := regexp.MustCompile(`(\w+)\s*(=~|!~|!=|=)\s*"([^"]*)"`)
			for _, lm := range labelRe.FindAllStringSubmatch(matches[2], -1) {
				if len(lm) > 3 {
					parsed.Filters = append(parsed.Filters, Filter{
						Column:   lm[1],
						Operator: lm[2],
						Value:    lm[3],
					})
				}
			}
		}

		// Parse range
		if len(matches) > 3 && matches[3] != "" {
			duration := parsePromQLDuration(matches[3])
			parsed.TimeRange = &TimeRange{
				Duration: duration,
			}
		}
	} else {
		// Fallback: Parse simple metric name (no labels)
		simpleRe := regexp.MustCompile(`([a-zA-Z_:][a-zA-Z0-9_:]*)\s*(?:\[(\d+[smhdwy])\])?$`)
		if matches := simpleRe.FindStringSubmatch(query); len(matches) > 1 {
			parsed.Sources = append(parsed.Sources, DataSource{
				Name: matches[1],
			})
			if len(matches) > 2 && matches[2] != "" {
				duration := parsePromQLDuration(matches[2])
				parsed.TimeRange = &TimeRange{
					Duration: duration,
				}
			}
		}
	}

	// Parse functions
	funcRe := regexp.MustCompile(`(rate|irate|increase|delta|deriv|sum|avg|min|max|count|stddev|topk|bottomk|quantile)\s*(?:\(|by|without)`)
	for _, match := range funcRe.FindAllStringSubmatch(query, -1) {
		if len(match) > 1 {
			parsed.Aggregations = append(parsed.Aggregations, Aggregation{
				Function: match[1],
			})
		}
	}

	// Parse by/without clauses
	byRe := regexp.MustCompile(`(?:sum|avg|min|max|count)\s+by\s*\(([^)]+)\)`)
	if matches := byRe.FindStringSubmatch(query); len(matches) > 1 {
		for _, label := range strings.Split(matches[1], ",") {
			parsed.GroupBy = append(parsed.GroupBy, strings.TrimSpace(label))
		}
	}

	return parsed, nil
}

func (h *PromQLHandler) FormatResponse(result *QueryResult, format string) (interface{}, error) {
	// Format as Prometheus response
	type sample struct {
		Metric map[string]string `json:"metric"`
		Value  []interface{}     `json:"value,omitempty"`
		Values [][]interface{}   `json:"values,omitempty"`
	}

	samples := make([]sample, 0)

	for _, row := range result.Rows {
		s := sample{
			Metric: make(map[string]string),
		}

		for i, col := range result.Columns {
			if col.IsTag {
				if v, ok := row[i].(string); ok {
					s.Metric[col.Name] = v
				}
			} else if col.IsTime {
				if len(row) > i+1 {
					s.Value = []interface{}{row[i], row[i+1]}
				}
			}
		}

		samples = append(samples, s)
	}

	return map[string]interface{}{
		"resultType": "vector",
		"result":     samples,
	}, nil
}

// SQLHandler handles generic SQL queries
type SQLHandler struct{}

func (h *SQLHandler) Dialect() Dialect {
	return DialectSQL
}

func (h *SQLHandler) Parse(query string) (*ParsedQuery, error) {
	query = strings.TrimSpace(query)
	upper := strings.ToUpper(query)

	parsed := &ParsedQuery{
		Dialect:       DialectSQL,
		OriginalQuery: query,
	}

	// Parse FROM
	fromRe := regexp.MustCompile(`(?i)FROM\s+(\w+)`)
	if matches := fromRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Sources = append(parsed.Sources, DataSource{
			Name: matches[1],
		})
	}

	// Parse LIMIT
	limitRe := regexp.MustCompile(`(?i)LIMIT\s+(\d+)`)
	if matches := limitRe.FindStringSubmatch(query); len(matches) > 1 {
		parsed.Limit, _ = strconv.ParseInt(matches[1], 10, 64)
	}

	// Parse ORDER BY
	orderRe := regexp.MustCompile(`(?i)ORDER\s+BY\s+(\w+)(?:\s+(ASC|DESC))?`)
	if matches := orderRe.FindStringSubmatch(query); len(matches) > 1 {
		ascending := true
		if len(matches) > 2 && strings.ToUpper(matches[2]) == "DESC" {
			ascending = false
		}
		parsed.OrderBy = append(parsed.OrderBy, OrderBy{
			Column:    matches[1],
			Ascending: ascending,
		})
	}

	_ = upper
	return parsed, nil
}

func (h *SQLHandler) FormatResponse(result *QueryResult, format string) (interface{}, error) {
	return formatAsJSON(result), nil
}

// Helper functions

func parseFluxDuration(s string) time.Duration {
	s = strings.TrimPrefix(s, "-")
	re := regexp.MustCompile(`(\d+)([smhd])`)
	if matches := re.FindStringSubmatch(s); len(matches) > 2 {
		value, _ := strconv.ParseInt(matches[1], 10, 64)
		switch matches[2] {
		case "s":
			return time.Duration(value) * time.Second
		case "m":
			return time.Duration(value) * time.Minute
		case "h":
			return time.Duration(value) * time.Hour
		case "d":
			return time.Duration(value) * 24 * time.Hour
		}
	}
	return 0
}

func parsePromQLDuration(s string) time.Duration {
	re := regexp.MustCompile(`(\d+)([smhdwy])`)
	if matches := re.FindStringSubmatch(s); len(matches) > 2 {
		value, _ := strconv.ParseInt(matches[1], 10, 64)
		switch matches[2] {
		case "s":
			return time.Duration(value) * time.Second
		case "m":
			return time.Duration(value) * time.Minute
		case "h":
			return time.Duration(value) * time.Hour
		case "d":
			return time.Duration(value) * 24 * time.Hour
		case "w":
			return time.Duration(value) * 7 * 24 * time.Hour
		case "y":
			return time.Duration(value) * 365 * 24 * time.Hour
		}
	}
	return 0
}

func formatAsJSON(result *QueryResult) interface{} {
	columns := make([]string, len(result.Columns))
	for i, col := range result.Columns {
		columns[i] = col.Name
	}

	return map[string]interface{}{
		"columns": columns,
		"rows":    result.Rows,
		"stats": map[string]interface{}{
			"execution_time_ms": result.Stats.ExecutionTimeMs,
			"rows_scanned":      result.Stats.RowsScanned,
			"bytes_scanned":     result.Stats.BytesScanned,
		},
	}
}

func formatAsCSV(result *QueryResult) string {
	var sb strings.Builder

	// Header
	for i, col := range result.Columns {
		if i > 0 {
			sb.WriteString(",")
		}
		sb.WriteString(col.Name)
	}
	sb.WriteString("\n")

	// Rows
	for _, row := range result.Rows {
		for i, val := range row {
			if i > 0 {
				sb.WriteString(",")
			}
			sb.WriteString(fmt.Sprintf("%v", val))
		}
		sb.WriteString("\n")
	}

	return sb.String()
}
