package tests

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/lumadb/cluster/pkg/tdengine"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Response matches TDengine response format
type Response struct {
	Code         int             `json:"code"`
	Desc         string          `json:"desc,omitempty"`
	ColumnMeta   [][]interface{} `json:"column_meta,omitempty"`
	Data         [][]interface{} `json:"data,omitempty"`
	Rows         int             `json:"rows,omitempty"`
	AffectedRows int             `json:"affected_rows,omitempty"`
}

func TestTDengineRESTAPI(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	t.Run("CreateDatabase", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS power PRECISION 'ms'")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("CreateSuperTable", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			CREATE STABLE power.meters (
				ts TIMESTAMP,
				current FLOAT,
				voltage INT,
				phase FLOAT
			) TAGS (location BINARY(64), groupid INT)
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("CreateSubTable", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			CREATE TABLE power.d1001 USING power.meters TAGS ('California.SanFrancisco', 2)
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("InsertData", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			INSERT INTO power.d1001 VALUES 
				(NOW, 10.3, 219, 0.31),
				(NOW + 1s, 10.5, 220, 0.32),
				(NOW + 2s, 10.7, 221, 0.33)
		`)
		assert.Equal(t, 0, resp.Code)
		assert.GreaterOrEqual(t, resp.AffectedRows, 1)
	})

	t.Run("InsertWithAutoCreate", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			INSERT INTO power.d1002 USING power.meters TAGS ('California.LosAngeles', 3) 
			VALUES (NOW, 12.5, 225, 0.35)
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("ShowDatabases", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW DATABASES")
		assert.Equal(t, 0, resp.Code)
		assert.GreaterOrEqual(t, resp.Rows, 1)
	})

	t.Run("ShowTables", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW TABLES FROM power")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("ShowStables", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW STABLES FROM power")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("DescribeTable", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "DESCRIBE power.meters")
		assert.Equal(t, 0, resp.Code)
		assert.Greater(t, resp.Rows, 0)
	})

	t.Run("SelectBasic", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			SELECT * FROM power.meters LIMIT 10
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("UseDatabase", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "USE power")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("DropTable", func(t *testing.T) {
		// Create a temp table first
		executeSQL(t, server, auth, "CREATE TABLE power.temp_table USING power.meters TAGS ('temp', 0)")
		resp := executeSQL(t, server, auth, "DROP TABLE IF EXISTS power.temp_table")
		assert.Equal(t, 0, resp.Code)
	})
}

func TestTDengineSQLWithDatabase(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	// Create database
	executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS test_db")

	t.Run("SQLWithDBInPath", func(t *testing.T) {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql/test_db",
			bytes.NewReader([]byte("SHOW TABLES")))
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		assert.Equal(t, 0, result.Code)
	})
}

func TestSchemalessIngestion(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	// Create database first
	executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS test")

	t.Run("InfluxDBLineProtocol", func(t *testing.T) {
		lines := `cpu,host=server01,region=us-west usage=0.64,idle=0.36 1626006833639000000
cpu,host=server02,region=us-east usage=0.55,idle=0.45 1626006833639000000`

		req, _ := http.NewRequest("POST", server.URL+"/influxdb/v1/write?db=test&precision=ns",
			bytes.NewReader([]byte(lines)))
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		assert.Equal(t, http.StatusNoContent, resp.StatusCode)
	})

	t.Run("InfluxDBWithDifferentPrecisions", func(t *testing.T) {
		testCases := []struct {
			precision string
			timestamp string
		}{
			{"ns", "1626006833639000000"},
			{"us", "1626006833639000"},
			{"ms", "1626006833639"},
			{"s", "1626006833"},
		}

		for _, tc := range testCases {
			t.Run(tc.precision, func(t *testing.T) {
				line := fmt.Sprintf("test,tag=value field=1.0 %s", tc.timestamp)
				req, _ := http.NewRequest("POST",
					server.URL+"/influxdb/v1/write?db=test&precision="+tc.precision,
					bytes.NewReader([]byte(line)))
				req.Header.Set("Authorization", "Basic "+auth)

				resp, err := http.DefaultClient.Do(req)
				require.NoError(t, err)
				resp.Body.Close()

				assert.Equal(t, http.StatusNoContent, resp.StatusCode)
			})
		}
	})

	t.Run("OpenTSDBJSON", func(t *testing.T) {
		points := []map[string]interface{}{
			{
				"metric":    "sys.cpu.nice",
				"timestamp": time.Now().UnixMilli(),
				"value":     18.5,
				"tags": map[string]string{
					"host": "web01",
					"dc":   "lga",
				},
			},
		}

		body, _ := json.Marshal(points)
		req, _ := http.NewRequest("POST", server.URL+"/opentsdb/v1/put/json/test",
			bytes.NewReader(body))
		req.Header.Set("Authorization", "Basic "+auth)
		req.Header.Set("Content-Type", "application/json")

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		assert.Equal(t, http.StatusNoContent, resp.StatusCode)
	})

	t.Run("OpenTSDBTelnet", func(t *testing.T) {
		lines := "put sys.cpu.user 1626006833 18.5 host=web01 dc=lga"

		req, _ := http.NewRequest("POST", server.URL+"/opentsdb/v1/put/telnet/test",
			bytes.NewReader([]byte(lines)))
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		assert.Equal(t, http.StatusNoContent, resp.StatusCode)
	})
}

func TestAuthentication(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	t.Run("ValidBasicAuth", func(t *testing.T) {
		auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))
		resp := executeSQL(t, server, auth, "SHOW DATABASES")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("InvalidBasicAuth", func(t *testing.T) {
		auth := base64.StdEncoding.EncodeToString([]byte("wrong:wrong"))
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql",
			bytes.NewReader([]byte("SHOW DATABASES")))
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		assert.NotEqual(t, 0, result.Code)
	})

	t.Run("NoAuth", func(t *testing.T) {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql",
			bytes.NewReader([]byte("SHOW DATABASES")))

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		assert.NotEqual(t, 0, result.Code)
	})

	t.Run("LoginEndpoint", func(t *testing.T) {
		auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))
		req, _ := http.NewRequest("GET", server.URL+"/rest/login/root/taosdata", nil)
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		assert.Equal(t, 0, result.Code)
		assert.NotEmpty(t, result.Desc) // Token returned in Desc
	})
}

func TestHealthEndpoints(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	t.Run("Health", func(t *testing.T) {
		resp, err := http.Get(server.URL + "/health")
		require.NoError(t, err)
		defer resp.Body.Close()

		assert.Equal(t, http.StatusOK, resp.StatusCode)
	})

	t.Run("Ready", func(t *testing.T) {
		resp, err := http.Get(server.URL + "/ready")
		require.NoError(t, err)
		defer resp.Body.Close()

		assert.Equal(t, http.StatusOK, resp.StatusCode)
	})
}

func TestSQLWithTiming(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	t.Run("SQLTiming", func(t *testing.T) {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sqlt",
			bytes.NewReader([]byte("SHOW DATABASES")))
		req.Header.Set("Authorization", "Basic "+auth)

		resp, err := http.DefaultClient.Do(req)
		require.NoError(t, err)
		defer resp.Body.Close()

		var result map[string]interface{}
		json.NewDecoder(resp.Body).Decode(&result)

		assert.Equal(t, float64(0), result["code"])
		assert.Contains(t, result, "timing")
	})
}

func TestDatabaseOperations(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	t.Run("CreateDatabaseWithOptions", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			CREATE DATABASE IF NOT EXISTS test_options 
			PRECISION 'us' 
			KEEP '365d,365d,365d'
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("DropDatabase", func(t *testing.T) {
		executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS to_drop")
		resp := executeSQL(t, server, auth, "DROP DATABASE IF EXISTS to_drop")
		assert.Equal(t, 0, resp.Code)
	})
}

func TestStreamOperations(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	// Setup
	executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS stream_test")
	executeSQL(t, server, auth, `
		CREATE STABLE stream_test.source (ts TIMESTAMP, value FLOAT) TAGS (id INT)
	`)
	executeSQL(t, server, auth, `
		CREATE TABLE stream_test.output (ts TIMESTAMP, avg_value FLOAT) 
	`)

	t.Run("CreateStream", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			CREATE STREAM test_stream INTO stream_test.output AS 
			SELECT _wstart as ts, AVG(value) as avg_value 
			FROM stream_test.source 
			INTERVAL(1m)
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("ShowStreams", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW STREAMS")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("DropStream", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "DROP STREAM IF EXISTS test_stream")
		assert.Equal(t, 0, resp.Code)
	})
}

func TestTopicOperations(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	// Setup
	executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS topic_test")
	executeSQL(t, server, auth, `
		CREATE STABLE topic_test.source (ts TIMESTAMP, value FLOAT) TAGS (id INT)
	`)

	t.Run("CreateTopic", func(t *testing.T) {
		resp := executeSQL(t, server, auth, `
			CREATE TOPIC test_topic AS SELECT * FROM topic_test.source
		`)
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("ShowTopics", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW TOPICS")
		assert.Equal(t, 0, resp.Code)
	})

	t.Run("DropTopic", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "DROP TOPIC IF EXISTS test_topic")
		assert.Equal(t, 0, resp.Code)
	})
}

func TestClusterInfo(t *testing.T) {
	server := setupTDengineServer(t)
	defer server.Close()

	auth := base64.StdEncoding.EncodeToString([]byte("root:taosdata"))

	t.Run("ShowDnodes", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW DNODES")
		assert.Equal(t, 0, resp.Code)
		assert.GreaterOrEqual(t, resp.Rows, 1)
	})

	t.Run("ShowMnodes", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW MNODES")
		assert.Equal(t, 0, resp.Code)
		assert.GreaterOrEqual(t, resp.Rows, 1)
	})

	t.Run("ShowUsers", func(t *testing.T) {
		resp := executeSQL(t, server, auth, "SHOW USERS")
		assert.Equal(t, 0, resp.Code)
		assert.GreaterOrEqual(t, resp.Rows, 1)
	})

	t.Run("ShowVgroups", func(t *testing.T) {
		executeSQL(t, server, auth, "CREATE DATABASE IF NOT EXISTS vgroup_test")
		resp := executeSQL(t, server, auth, "SHOW VGROUPS FROM vgroup_test")
		assert.Equal(t, 0, resp.Code)
	})
}

// Helper functions

func executeSQL(t *testing.T, server *httptest.Server, auth, sql string) *Response {
	req, err := http.NewRequest("POST", server.URL+"/rest/sql", bytes.NewReader([]byte(sql)))
	require.NoError(t, err)

	req.Header.Set("Authorization", "Basic "+auth)

	resp, err := http.DefaultClient.Do(req)
	require.NoError(t, err)
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	var result Response
	json.Unmarshal(body, &result)

	return &result
}

func setupTDengineServer(t *testing.T) *httptest.Server {
	// Initialize LumaDB with TDengine compatibility layer
	engine := tdengine.NewEngine()
	api := tdengine.NewAPI(engine)

	mux := http.NewServeMux()
	api.Register(mux)

	return httptest.NewServer(mux)
}
