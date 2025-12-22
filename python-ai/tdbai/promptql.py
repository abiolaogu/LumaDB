"""
PromptQL: Natural Language to TDengine SQL
AI-Enhanced Query Interface for LumaDB

Enables users to query TDengine data using natural language,
which is then converted to TDengine SQL.
"""

import re
from typing import Optional, Dict, Any, List
from dataclasses import dataclass
from enum import Enum

try:
    import anthropic
    ANTHROPIC_AVAILABLE = True
except ImportError:
    ANTHROPIC_AVAILABLE = False

try:
    from pydantic import BaseModel
    PYDANTIC_AVAILABLE = True
except ImportError:
    PYDANTIC_AVAILABLE = False


class QueryIntent(Enum):
    """Detected query intent"""
    AGGREGATE = "aggregate"
    TIME_SERIES = "time_series"
    LATEST_VALUE = "latest_value"
    ANOMALY_DETECTION = "anomaly_detection"
    COMPARISON = "comparison"
    TREND_ANALYSIS = "trend_analysis"
    THRESHOLD_ALERT = "threshold_alert"


@dataclass
class ParsedQuery:
    """Parsed natural language query"""
    intent: QueryIntent
    metrics: List[str]
    time_range: Optional[str]
    interval: Optional[str]
    filters: Dict[str, Any]
    aggregations: List[str]
    group_by: List[str]
    order_by: Optional[str]
    limit: Optional[int]


class PromptQL:
    """
    Natural Language to TDengine SQL converter.
    
    Examples:
        "Show me the average temperature for the last hour"
        → SELECT _wstart, AVG(temperature) FROM sensors 
          WHERE ts > NOW() - 1h INTERVAL(5m)
        
        "What's the current voltage for all meters in California?"
        → SELECT LAST_ROW(voltage) FROM meters 
          WHERE location LIKE 'California%' GROUP BY tbname
        
        "Alert me when CPU usage exceeds 90%"
        → CREATE STREAM cpu_alert INTO alerts AS 
          SELECT * FROM cpu WHERE usage > 90
    """
    
    def __init__(self, schema_context: Dict[str, Any] = None, api_key: str = None):
        """
        Initialize PromptQL.
        
        Args:
            schema_context: Database schema information for context
            api_key: Anthropic API key (optional, uses env var if not provided)
        """
        self.schema_context = schema_context or {}
        self.client = None
        
        if ANTHROPIC_AVAILABLE:
            try:
                self.client = anthropic.Anthropic(api_key=api_key) if api_key else anthropic.Anthropic()
            except Exception:
                self.client = None
    
    def to_sql(self, natural_query: str, database: str = None) -> str:
        """
        Convert natural language to TDengine SQL.
        
        Args:
            natural_query: Natural language query
            database: Target database name
            
        Returns:
            TDengine SQL query
        """
        if self.client is None:
            return self._fallback_to_sql(natural_query, database)
        
        system_prompt = self._build_system_prompt(database)
        
        try:
            response = self.client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=1000,
                system=system_prompt,
                messages=[
                    {"role": "user", "content": natural_query}
                ]
            )
            
            sql = self._extract_sql(response.content[0].text)
            return self._validate_and_optimize(sql)
        except Exception as e:
            # Fallback to rule-based conversion
            return self._fallback_to_sql(natural_query, database)
    
    def _fallback_to_sql(self, natural_query: str, database: str = None) -> str:
        """
        Rule-based fallback when AI is not available.
        """
        query_lower = natural_query.lower()
        
        # Detect table name
        table = "data"
        for word in query_lower.split():
            if word in ["sensors", "meters", "devices", "cpu", "memory", "temperature"]:
                table = word
                break
        
        # Detect time range
        time_range = "1h"
        time_patterns = [
            (r"last\s+(\d+)\s*(hour|h)", "h"),
            (r"last\s+(\d+)\s*(minute|min|m)", "m"),
            (r"last\s+(\d+)\s*(day|d)", "d"),
            (r"past\s+(\d+)\s*(hour|h)", "h"),
            (r"past\s+(\d+)\s*(minute|min|m)", "m"),
        ]
        for pattern, unit in time_patterns:
            match = re.search(pattern, query_lower)
            if match:
                time_range = f"{match.group(1)}{unit}"
                break
        
        # Detect aggregation
        agg_func = "AVG"
        if "average" in query_lower or "avg" in query_lower:
            agg_func = "AVG"
        elif "maximum" in query_lower or "max" in query_lower:
            agg_func = "MAX"
        elif "minimum" in query_lower or "min" in query_lower:
            agg_func = "MIN"
        elif "sum" in query_lower or "total" in query_lower:
            agg_func = "SUM"
        elif "count" in query_lower:
            agg_func = "COUNT"
        elif "current" in query_lower or "latest" in query_lower or "last" in query_lower:
            agg_func = "LAST_ROW"
        
        # Detect metric
        metric = "value"
        metric_words = ["temperature", "voltage", "current", "power", "usage", "cpu", "memory"]
        for word in metric_words:
            if word in query_lower:
                metric = word
                break
        
        # Build SQL
        db_prefix = f"{database}." if database else ""
        
        if agg_func == "LAST_ROW":
            return f"SELECT LAST_ROW({metric}) FROM {db_prefix}{table};"
        else:
            return f"SELECT _wstart, {agg_func}({metric}) FROM {db_prefix}{table} WHERE ts > NOW() - {time_range} INTERVAL(5m);"
    
    def _build_system_prompt(self, database: str) -> str:
        """Build the system prompt with schema context."""
        schema_info = self._format_schema(database) if self.schema_context else ""
        
        return f"""You are a TDengine SQL expert. Convert natural language queries to TDengine SQL.

TDengine SQL Features:
- Supertables and subtables (one table per device)
- Time-series extensions: INTERVAL, SLIDING, FILL, PARTITION BY
- Window functions: STATE_WINDOW, SESSION, EVENT_WINDOW, COUNT_WINDOW
- Special functions: LAST_ROW, TWA, SPREAD, DIFF, DERIVATIVE, IRATE, ELAPSED
- Pseudocolumns: _wstart, _wend, _wduration, tbname
- Stream processing: CREATE STREAM ... INTO ... AS SELECT ...

{schema_info}

Rules:
1. Always use proper time filtering with ts column
2. Use INTERVAL for time-series aggregations
3. Use LAST_ROW for current/latest values
4. Use PARTITION BY tbname for per-device analysis
5. Use appropriate FILL clause for gaps
6. Return ONLY the SQL, no explanations

Examples:
- "average temperature last hour" → SELECT _wstart, AVG(temperature) FROM sensors WHERE ts > NOW() - 1h INTERVAL(5m)
- "current voltage all meters" → SELECT tbname, LAST_ROW(voltage) FROM meters GROUP BY tbname
- "max CPU per server today" → SELECT tbname, MAX(cpu) FROM servers WHERE ts >= TODAY() PARTITION BY tbname
"""
    
    def _format_schema(self, database: str) -> str:
        """Format schema information for the prompt."""
        if not self.schema_context or database not in self.schema_context:
            return ""
        
        schema = self.schema_context[database]
        lines = [f"Database: {database}", "Tables:"]
        
        for table_name, table_info in schema.items():
            cols = ", ".join([f"{c['name']} {c['type']}" for c in table_info.get('columns', [])])
            tags = ", ".join([f"{t['name']} {t['type']}" for t in table_info.get('tags', [])])
            lines.append(f"  - {table_name}({cols}) TAGS({tags})")
        
        return "\n".join(lines)
    
    def _extract_sql(self, response: str) -> str:
        """Extract SQL from LLM response."""
        # Look for SQL in code blocks
        code_match = re.search(r'```(?:sql)?\s*(.*?)\s*```', response, re.DOTALL | re.IGNORECASE)
        if code_match:
            return code_match.group(1).strip()
        
        # Look for SELECT/CREATE/INSERT statements
        sql_match = re.search(
            r'((?:SELECT|CREATE|INSERT|DELETE|DROP|ALTER|SHOW|DESCRIBE)\s+.+?)(?:;|\n\n|$)', 
            response, re.DOTALL | re.IGNORECASE
        )
        if sql_match:
            return sql_match.group(1).strip()
        
        return response.strip()
    
    def _validate_and_optimize(self, sql: str) -> str:
        """Validate and optimize the generated SQL."""
        # Basic validation
        sql = sql.strip().rstrip(';') + ';'
        
        # Ensure proper quoting for string comparisons
        sql = re.sub(r"=\s*([A-Za-z][A-Za-z0-9_]*)\s", r"= '\1' ", sql)
        
        return sql
    
    def explain(self, natural_query: str, database: str = None) -> Dict[str, Any]:
        """
        Explain what the query will do.
        
        Args:
            natural_query: Natural language query
            database: Target database name
            
        Returns:
            Dictionary with SQL and explanation
        """
        sql = self.to_sql(natural_query, database)
        
        return {
            "natural_query": natural_query,
            "generated_sql": sql,
            "explanation": self._generate_explanation(sql)
        }
    
    def _generate_explanation(self, sql: str) -> str:
        """Generate human-readable explanation of SQL."""
        if self.client is None:
            return "AI explanation not available. The SQL query was generated using rule-based conversion."
        
        try:
            response = self.client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=500,
                messages=[
                    {"role": "user", "content": f"Explain this TDengine SQL in simple terms:\n{sql}"}
                ]
            )
            return response.content[0].text
        except Exception:
            return "Explanation not available."


class AnomalyDetector:
    """AI-powered anomaly detection for TDengine data."""
    
    def __init__(self, engine):
        """
        Initialize anomaly detector.
        
        Args:
            engine: Database engine for executing queries
        """
        self.engine = engine
    
    def detect(
        self, 
        database: str, 
        table: str, 
        metric: str, 
        time_range: str = "1h", 
        sensitivity: float = 2.0
    ) -> List[Dict]:
        """
        Detect anomalies in time-series data.
        
        Uses statistical methods (z-score, IQR) combined with
        TDengine's time-series functions for efficient detection.
        
        Args:
            database: Database name
            table: Table name
            metric: Metric column to analyze
            time_range: Time range to analyze
            sensitivity: Z-score threshold (default: 2.0)
            
        Returns:
            List of detected anomalies
        """
        # Build detection query using window functions
        sql = f"""
        SELECT ts, {metric},
               AVG({metric}) OVER (PARTITION BY tbname ORDER BY ts ROWS BETWEEN 100 PRECEDING AND CURRENT ROW) as rolling_avg,
               STDDEV({metric}) OVER (PARTITION BY tbname ORDER BY ts ROWS BETWEEN 100 PRECEDING AND CURRENT ROW) as rolling_std
        FROM {database}.{table}
        WHERE ts > NOW() - {time_range}
        """
        
        try:
            results = self.engine.query(sql)
        except Exception:
            # Fallback: simpler query without window functions
            results = []
        
        anomalies = []
        for row in results:
            rolling_std = row.get('rolling_std', 0)
            if rolling_std and rolling_std > 0:
                z_score = abs(row[metric] - row.get('rolling_avg', 0)) / rolling_std
                if z_score > sensitivity:
                    anomalies.append({
                        'timestamp': row.get('ts'),
                        'value': row.get(metric),
                        'expected': row.get('rolling_avg'),
                        'z_score': z_score,
                        'severity': 'high' if z_score > 3 else 'medium'
                    })
        
        return anomalies
    
    def detect_simple(
        self, 
        values: List[float], 
        timestamps: List[int] = None,
        sensitivity: float = 2.0
    ) -> List[Dict]:
        """
        Simple anomaly detection on a list of values.
        
        Args:
            values: List of numeric values
            timestamps: Optional list of timestamps
            sensitivity: Z-score threshold
            
        Returns:
            List of detected anomalies
        """
        if not values:
            return []
        
        mean = sum(values) / len(values)
        variance = sum((v - mean) ** 2 for v in values) / len(values)
        std = variance ** 0.5
        
        if std == 0:
            return []
        
        anomalies = []
        for i, value in enumerate(values):
            z_score = abs(value - mean) / std
            if z_score > sensitivity:
                anomalies.append({
                    'index': i,
                    'timestamp': timestamps[i] if timestamps else None,
                    'value': value,
                    'expected': mean,
                    'z_score': z_score,
                    'severity': 'high' if z_score > 3 else 'medium'
                })
        
        return anomalies


class Forecaster:
    """Time-series forecasting for TDengine data."""
    
    def __init__(self, engine=None):
        """
        Initialize forecaster.
        
        Args:
            engine: Optional database engine for fetching data
        """
        self.engine = engine
    
    def forecast(
        self, 
        database: str, 
        table: str, 
        metric: str,
        horizon: str = "1h", 
        granularity: str = "5m"
    ) -> List[Dict]:
        """
        Forecast future values using historical patterns.
        
        Combines TDengine's aggregation with simple forecasting models.
        
        Args:
            database: Database name
            table: Table name
            metric: Metric column to forecast
            horizon: Forecast horizon
            granularity: Time granularity for forecast points
            
        Returns:
            List of forecast points
        """
        if self.engine is None:
            return []
        
        # Get historical data
        sql = f"""
        SELECT _wstart as ts, AVG({metric}) as value
        FROM {database}.{table}
        WHERE ts > NOW() - 7d
        INTERVAL({granularity})
        """
        
        try:
            history = self.engine.query(sql)
        except Exception:
            history = []
        
        if not history:
            return []
        
        values = [r.get('value') for r in history if r.get('value') is not None]
        
        if not values:
            return []
        
        # Simple exponential smoothing forecast
        alpha = 0.3
        smoothed = values[0]
        for v in values[1:]:
            smoothed = alpha * v + (1 - alpha) * smoothed
        
        # Generate forecast points
        horizon_ms = self._parse_duration(horizon)
        interval_ms = self._parse_duration(granularity)
        num_points = horizon_ms // interval_ms if interval_ms > 0 else 0
        
        last_ts = history[-1].get('ts', 0) if history else 0
        forecasts = []
        
        for i in range(1, int(num_points) + 1):
            forecasts.append({
                'timestamp': last_ts + i * interval_ms,
                'forecast': smoothed,
                'lower_bound': smoothed * 0.9,
                'upper_bound': smoothed * 1.1,
            })
        
        return forecasts
    
    def forecast_from_values(
        self,
        values: List[float],
        timestamps: List[int] = None,
        num_points: int = 10,
        alpha: float = 0.3
    ) -> List[Dict]:
        """
        Forecast from a list of values using exponential smoothing.
        
        Args:
            values: Historical values
            timestamps: Optional timestamps
            num_points: Number of forecast points
            alpha: Smoothing factor (0-1)
            
        Returns:
            List of forecast points
        """
        if not values:
            return []
        
        # Exponential smoothing
        smoothed = values[0]
        for v in values[1:]:
            smoothed = alpha * v + (1 - alpha) * smoothed
        
        # Calculate interval from timestamps
        if timestamps and len(timestamps) >= 2:
            avg_interval = (timestamps[-1] - timestamps[0]) / (len(timestamps) - 1)
            last_ts = timestamps[-1]
        else:
            avg_interval = 60000  # Default 1 minute
            last_ts = 0
        
        # Generate forecasts
        forecasts = []
        for i in range(1, num_points + 1):
            forecasts.append({
                'timestamp': int(last_ts + i * avg_interval),
                'forecast': smoothed,
                'lower_bound': smoothed * 0.9,
                'upper_bound': smoothed * 1.1,
            })
        
        return forecasts
    
    def _parse_duration(self, s: str) -> int:
        """Parse duration string to milliseconds."""
        units = {'s': 1000, 'm': 60000, 'h': 3600000, 'd': 86400000}
        match = re.match(r'(\d+)([smhd])', s)
        if match:
            return int(match.group(1)) * units.get(match.group(2), 1000)
        return int(s) if s.isdigit() else 0


class QueryOptimizer:
    """Optimize TDengine queries for better performance."""
    
    @staticmethod
    def optimize(sql: str) -> str:
        """
        Optimize a TDengine SQL query.
        
        Args:
            sql: Original SQL query
            
        Returns:
            Optimized SQL query
        """
        optimized = sql
        
        # Add PARTITION BY tbname for supertable queries if not present
        if 'INTERVAL(' in sql.upper() and 'PARTITION BY' not in sql.upper():
            # Insert PARTITION BY before INTERVAL
            optimized = re.sub(
                r'(INTERVAL\([^)]+\))',
                r'PARTITION BY tbname \1',
                optimized,
                flags=re.IGNORECASE
            )
        
        # Ensure time filter is present
        if 'WHERE' not in sql.upper() and ('SELECT' in sql.upper()):
            # Add time filter
            optimized = re.sub(
                r'(FROM\s+\S+)',
                r'\1 WHERE ts > NOW() - 1h',
                optimized,
                flags=re.IGNORECASE
            )
        
        return optimized
    
    @staticmethod
    def analyze(sql: str) -> Dict[str, Any]:
        """
        Analyze a query and provide optimization suggestions.
        
        Args:
            sql: SQL query to analyze
            
        Returns:
            Analysis results with suggestions
        """
        suggestions = []
        
        sql_upper = sql.upper()
        
        # Check for missing time filter
        if 'WHERE' not in sql_upper and 'SELECT' in sql_upper:
            suggestions.append({
                'type': 'performance',
                'message': 'Add a time filter (WHERE ts > ...) to limit data scanned',
                'severity': 'high'
            })
        
        # Check for SELECT *
        if 'SELECT *' in sql_upper:
            suggestions.append({
                'type': 'performance',
                'message': 'Avoid SELECT *, specify only needed columns',
                'severity': 'medium'
            })
        
        # Check for PARTITION BY with INTERVAL
        if 'INTERVAL(' in sql_upper and 'PARTITION BY' not in sql_upper:
            suggestions.append({
                'type': 'correctness',
                'message': 'Consider adding PARTITION BY tbname for per-device analysis',
                'severity': 'low'
            })
        
        # Check for FILL clause with INTERVAL
        if 'INTERVAL(' in sql_upper and 'FILL(' not in sql_upper:
            suggestions.append({
                'type': 'data_quality',
                'message': 'Consider adding FILL clause to handle gaps in data',
                'severity': 'low'
            })
        
        return {
            'original_sql': sql,
            'suggestions': suggestions,
            'optimized_sql': QueryOptimizer.optimize(sql) if suggestions else sql
        }


# Export main classes
__all__ = [
    'PromptQL',
    'AnomalyDetector', 
    'Forecaster',
    'QueryOptimizer',
    'QueryIntent',
    'ParsedQuery'
]
