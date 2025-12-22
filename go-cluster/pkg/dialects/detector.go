package dialects

import (
	"regexp"
	"strings"
)

// DialectDetector detects query dialects from query text
type DialectDetector struct {
	patterns map[Dialect][]*regexp.Regexp
	keywords map[Dialect][]string
}

// NewDialectDetector creates a new dialect detector
func NewDialectDetector() *DialectDetector {
	d := &DialectDetector{
		patterns: make(map[Dialect][]*regexp.Regexp),
		keywords: make(map[Dialect][]string),
	}
	d.initPatterns()
	d.initKeywords()
	return d
}

func (d *DialectDetector) initPatterns() {
	// InfluxQL patterns
	d.patterns[DialectInfluxQL] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)SELECT\s+.+\s+FROM\s+["\w]+\s+(WHERE|GROUP BY|LIMIT|ORDER BY|FILL|TZ)`),
		regexp.MustCompile(`(?i)\s+GROUP\s+BY\s+time\s*\(`),
		regexp.MustCompile(`(?i)SHOW\s+(MEASUREMENTS|TAG\s+KEYS|TAG\s+VALUES|FIELD\s+KEYS|DATABASES|RETENTION\s+POLICIES|SERIES)`),
		regexp.MustCompile(`(?i)CREATE\s+(DATABASE|RETENTION\s+POLICY|CONTINUOUS\s+QUERY)`),
	}

	// Flux patterns
	d.patterns[DialectFlux] = []*regexp.Regexp{
		regexp.MustCompile(`from\s*\(\s*bucket\s*:`),
		regexp.MustCompile(`\|>\s*range\s*\(`),
		regexp.MustCompile(`\|>\s*filter\s*\(`),
		regexp.MustCompile(`\|>\s*aggregateWindow\s*\(`),
		regexp.MustCompile(`\|>\s*yield\s*\(`),
		regexp.MustCompile(`\|>\s*map\s*\(`),
	}

	// PromQL patterns
	d.patterns[DialectPromQL] = []*regexp.Regexp{
		regexp.MustCompile(`\w+\s*\{[^}]*\}\s*(\[[\w]+\])?`),
		regexp.MustCompile(`(rate|irate|increase|delta|deriv|predict_linear|histogram_quantile)\s*\(`),
		regexp.MustCompile(`(sum|avg|min|max|count|stddev|topk|bottomk|quantile)\s*(by|without)\s*\(`),
		regexp.MustCompile(`\s+offset\s+\d+[smhdwy]`),
		regexp.MustCompile(`\[\d+[smhdwy]\]`),
	}

	// TDengine patterns
	d.patterns[DialectTDengine] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)CREATE\s+STABLE`),
		regexp.MustCompile(`(?i)USING\s+\w+\s+TAGS\s*\(`),
		regexp.MustCompile(`(?i)INTERVAL\s*\(\s*\d+[smhd]\s*\)`),
		regexp.MustCompile(`(?i)PARTITION\s+BY\s+TBNAME`),
		regexp.MustCompile(`(?i)(STATE_WINDOW|SESSION|EVENT_WINDOW|COUNT_WINDOW)\s*\(`),
		regexp.MustCompile(`(?i)LAST_ROW\s*\(`),
	}

	// TimescaleDB patterns
	d.patterns[DialectTimescale] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)time_bucket\s*\(`),
		regexp.MustCompile(`(?i)time_bucket_gapfill\s*\(`),
		regexp.MustCompile(`(?i)CREATE\s+HYPERTABLE`),
		regexp.MustCompile(`(?i)(locf|interpolate)\s*\(`),
	}

	// QuestDB patterns
	d.patterns[DialectQuestDB] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)SAMPLE\s+BY`),
		regexp.MustCompile(`(?i)LATEST\s+ON`),
		regexp.MustCompile(`(?i)ASOF\s+JOIN`),
		regexp.MustCompile(`(?i)(LT|SPLICE)\s+JOIN`),
	}

	// ClickHouse patterns
	d.patterns[DialectClickHouse] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)ENGINE\s*=\s*(MergeTree|ReplacingMergeTree|SummingMergeTree|AggregatingMergeTree)`),
		regexp.MustCompile(`(?i)(toDateTime|toDate|toStartOfHour|toStartOfDay)\s*\(`),
		regexp.MustCompile(`(?i)arrayJoin\s*\(`),
		regexp.MustCompile(`(?i)WITH\s+TOTALS`),
		regexp.MustCompile(`(?i)PREWHERE`),
		regexp.MustCompile(`(?i)GLOBAL\s+(IN|JOIN)`),
	}

	// Druid SQL patterns
	d.patterns[DialectDruidSQL] = []*regexp.Regexp{
		regexp.MustCompile(`(?i)__time`),
		regexp.MustCompile(`(?i)FLOOR\s*\(\s*__time`),
		regexp.MustCompile(`(?i)TIME_FLOOR\s*\(`),
		regexp.MustCompile(`(?i)APPROX_COUNT_DISTINCT\s*\(`),
	}

	// Druid Native JSON patterns
	d.patterns[DialectDruidJSON] = []*regexp.Regexp{
		regexp.MustCompile(`"queryType"\s*:\s*"(timeseries|topN|groupBy|scan|search)"`),
		regexp.MustCompile(`"dataSource"\s*:`),
		regexp.MustCompile(`"granularity"\s*:`),
	}

	// OpenTSDB patterns
	d.patterns[DialectOpenTSDB] = []*regexp.Regexp{
		regexp.MustCompile(`"queries"\s*:\s*\[`),
		regexp.MustCompile(`"metric"\s*:\s*"`),
		regexp.MustCompile(`"aggregator"\s*:\s*"(sum|avg|min|max|count)"`),
	}

	// Graphite patterns
	d.patterns[DialectGraphite] = []*regexp.Regexp{
		regexp.MustCompile(`(summarize|derivative|integral|movingAverage|alias)\s*\(`),
		regexp.MustCompile(`\*\.\*\.`),
	}

	// MetricsQL patterns (PromQL superset)
	d.patterns[DialectMetricsQL] = []*regexp.Regexp{
		regexp.MustCompile(`(range_quantile|range_median|range_avg|range_first|range_last)\s*\(`),
		regexp.MustCompile(`(topk_avg|topk_max|topk_min|bottomk_avg)\s*\(`),
	}
}

func (d *DialectDetector) initKeywords() {
	d.keywords[DialectInfluxQL] = []string{
		"FILL(", "SLIMIT", "SOFFSET", "TZ(", "INTO",
		"SHOW MEASUREMENTS", "SHOW TAG", "SHOW FIELD",
		"GROUP BY time(",
	}

	d.keywords[DialectFlux] = []string{
		"|>", "from(bucket:", "range(", "filter(fn:",
		"aggregateWindow(", "map(fn:", "pivot(",
	}

	d.keywords[DialectPromQL] = []string{
		"rate(", "irate(", "increase(", "histogram_quantile(",
		"sum by", "sum without", "avg by", "count by",
		"__name__", "job=", "instance=",
	}

	d.keywords[DialectTDengine] = []string{
		"CREATE STABLE", "USING", "TAGS(", "INTERVAL(",
		"PARTITION BY", "STATE_WINDOW", "SESSION(",
		"LAST_ROW(", "TWA(", "SPREAD(", "_wstart", "_wend",
		"FILL(PREV)", "FILL(LINEAR)", "TBNAME",
	}

	d.keywords[DialectTimescale] = []string{
		"time_bucket(", "time_bucket_gapfill(",
		"CREATE HYPERTABLE", "locf(", "interpolate(",
		"add_retention_policy", "add_compression_policy",
	}

	d.keywords[DialectQuestDB] = []string{
		"SAMPLE BY", "LATEST ON", "ASOF JOIN",
		"LT JOIN", "SPLICE JOIN", "designated timestamp",
	}

	d.keywords[DialectClickHouse] = []string{
		"MergeTree", "ReplacingMergeTree", "ENGINE=",
		"toDateTime(", "toStartOfHour(", "arrayJoin(",
		"PREWHERE", "GLOBAL IN", "WITH TOTALS", "FINAL",
	}

	d.keywords[DialectDruidSQL] = []string{
		"__time", "TIME_FLOOR(", "TIME_SHIFT(",
		"APPROX_COUNT_DISTINCT(", "DS_HLL", "DS_THETA",
	}

	d.keywords[DialectGraphite] = []string{
		"summarize(", "alias(", "scale(", "offset(",
		"derivative(", "integral(", "movingAverage(",
	}
}

// Detect detects the dialect of a query and returns confidence score
func (d *DialectDetector) Detect(query string) (Dialect, float64) {
	query = strings.TrimSpace(query)

	// Check if JSON (Druid native, OpenTSDB)
	if strings.HasPrefix(query, "{") || strings.HasPrefix(query, "[") {
		if d.matchesPatterns(query, DialectDruidJSON) {
			return DialectDruidJSON, 0.9
		}
		if d.matchesPatterns(query, DialectOpenTSDB) {
			return DialectOpenTSDB, 0.9
		}
	}

	// Score each dialect
	scores := make(map[Dialect]int)

	// Check patterns
	for dialect, patterns := range d.patterns {
		for _, pattern := range patterns {
			if pattern.MatchString(query) {
				scores[dialect] += 10
			}
		}
	}

	// Check keywords
	queryUpper := strings.ToUpper(query)
	for dialect, keywords := range d.keywords {
		for _, keyword := range keywords {
			if strings.Contains(queryUpper, strings.ToUpper(keyword)) {
				scores[dialect] += 5
			}
		}
	}

	// Find highest scoring dialect
	var bestDialect Dialect = DialectSQL
	var bestScore int

	for dialect, score := range scores {
		if score > bestScore {
			bestScore = score
			bestDialect = dialect
		}
	}

	// Calculate confidence
	totalScore := 0
	for _, score := range scores {
		totalScore += score
	}

	var confidence float64
	if totalScore > 0 && bestScore >= 5 {
		confidence = float64(bestScore) / float64(totalScore)
	}

	// Fallback heuristics
	if bestScore < 5 {
		if strings.HasPrefix(queryUpper, "SELECT") || strings.HasPrefix(queryUpper, "SHOW") {
			return DialectSQL, 0.3
		}

		// Check for PromQL-style metric selector
		promqlRe := regexp.MustCompile(`^[a-zA-Z_:][a-zA-Z0-9_:]*(\{.*\})?(\[.*\])?$`)
		if promqlRe.MatchString(query) {
			return DialectPromQL, 0.5
		}

		return DialectSQL, 0.1
	}

	return bestDialect, confidence
}

func (d *DialectDetector) matchesPatterns(query string, dialect Dialect) bool {
	patterns, ok := d.patterns[dialect]
	if !ok {
		return false
	}

	for _, pattern := range patterns {
		if pattern.MatchString(query) {
			return true
		}
	}
	return false
}
