# LumaDB Complete How-To Implementation Guide

## Step-by-Step Implementation Guide for All Features

**Version:** 3.0.0 | **Last Updated:** December 2024

---

## Table of Contents

1. [Installation & Setup](#1-installation--setup)
2. [Basic Database Operations](#2-basic-database-operations)
3. [Query Language Usage](#3-query-language-usage)
4. [Indexing & Performance](#4-indexing--performance)
5. [AI & Vector Search](#5-ai--vector-search)
6. [Time-Series Operations](#6-time-series-operations)
7. [GraphQL & REST APIs](#7-graphql--rest-apis)
8. [Security Configuration](#8-security-configuration)
9. [Cluster Deployment](#9-cluster-deployment)
10. [Monitoring & Observability](#10-monitoring--observability)
11. [Backup & Recovery](#11-backup--recovery)
12. [Migration Guides](#12-migration-guides)

---

## 1. Installation & Setup

### 1.1 Quick Start with Docker

```bash
# Single command deployment
docker run -d \
  --name lumadb \
  -p 8080:8080 \
  -p 5432:5432 \
  -p 9090:9090 \
  -v lumadb-data:/var/lib/lumadb \
  lumadb/lumadb:latest

# Verify installation
curl http://localhost:8080/health
```

### 1.2 Docker Compose (Full Stack)

```yaml
# docker-compose.yml
version: '3.8'

services:
  lumadb:
    image: lumadb/lumadb:latest
    ports:
      - "8080:8080"     # HTTP/REST/GraphQL
      - "5432:5432"     # PostgreSQL protocol
      - "3306:3306"     # MySQL protocol
      - "27017:27017"   # MongoDB protocol
      - "6379:6379"     # Redis protocol
      - "9090:9090"     # Prometheus metrics
    volumes:
      - ./data:/var/lib/lumadb
      - ./config:/etc/lumadb
    environment:
      LUMADB_ADMIN_SECRET: "your-admin-secret"
      LUMADB_JWT_SECRET: "your-jwt-secret"
      LUMADB_LOG_LEVEL: "info"
    restart: unless-stopped

  admin-ui:
    image: lumadb/admin-ui:latest
    ports:
      - "3000:3000"
    environment:
      LUMADB_ENDPOINT: "http://lumadb:8080"
    depends_on:
      - lumadb
```

```bash
# Start the stack
docker-compose up -d

# Check logs
docker-compose logs -f lumadb
```

### 1.3 Kubernetes Deployment

```bash
# Create namespace
kubectl create namespace lumadb

# Apply configurations
kubectl apply -f k8s/configmap.yaml -n lumadb
kubectl apply -f k8s/statefulset.yaml -n lumadb
kubectl apply -f k8s/service.yaml -n lumadb

# Verify deployment
kubectl get pods -n lumadb
kubectl get svc -n lumadb
```

### 1.4 TypeScript SDK Installation

```bash
# Install SDK
npm install tdb-plus

# Or with yarn
yarn add tdb-plus
```

### 1.5 Configuration File

```toml
# /etc/lumadb/config.toml

[server]
http_port = 8080
grpc_port = 50051
admin_ui_port = 3000

[storage]
data_dir = "/var/lib/lumadb/data"
wal_dir = "/var/lib/lumadb/wal"
compression = "lz4"

[memory]
memtable_size = 67108864      # 64 MB
block_cache_size = 134217728  # 128 MB
row_cache_size = 67108864     # 64 MB

[wal]
enabled = true
sync_mode = "group_commit"
batch_size = 1000

[compaction]
style = "leveled"
max_background_jobs = 4

[cluster]
node_id = "node-1"
raft_port = 10000
peers = []

[security]
require_auth = true
jwt_secret = "${JWT_SECRET}"
admin_secret = "${ADMIN_SECRET}"

[protocols]
postgres_enabled = true
postgres_port = 5432
mysql_enabled = true
mysql_port = 3306
mongodb_enabled = true
mongodb_port = 27017
```

---

## 2. Basic Database Operations

### 2.1 Connecting to LumaDB

**TypeScript/JavaScript:**
```typescript
import { Database } from 'tdb-plus';

// Create and open database
const db = Database.create('my_application');
await db.open();

// With configuration
const db = Database.create('my_application', {
  storage: 'file',
  path: './data',
  cache: {
    enabled: true,
    maxSize: 1000
  }
});
await db.open();
```

**PostgreSQL Protocol:**
```bash
psql -h localhost -p 5432 -U admin -d lumadb
```

**Python:**
```python
import psycopg2

conn = psycopg2.connect(
    host="localhost",
    port=5432,
    database="lumadb",
    user="admin",
    password="your-password"
)
cursor = conn.cursor()
```

### 2.2 Creating Collections

```typescript
// Get or create collection
const users = db.collection('users');

// With schema validation
const products = db.collection('products', {
  schema: {
    name: { type: 'string', required: true },
    price: { type: 'number', min: 0 },
    category: { type: 'string', enum: ['electronics', 'clothing', 'food'] },
    inStock: { type: 'boolean', default: true }
  }
});
```

### 2.3 CRUD Operations

**Insert:**
```typescript
// Single insert
const user = await users.insert({
  name: 'Alice Johnson',
  email: 'alice@example.com',
  age: 28,
  role: 'developer'
});
console.log('Created user:', user._id);

// Batch insert
const newUsers = await users.insertMany([
  { name: 'Bob Smith', email: 'bob@example.com', age: 32 },
  { name: 'Carol White', email: 'carol@example.com', age: 25 },
  { name: 'David Brown', email: 'david@example.com', age: 41 }
]);
console.log(`Created ${newUsers.length} users`);
```

**Read:**
```typescript
// Find by ID
const user = await users.findById('user-123');

// Find with conditions
const activeUsers = await users.find({
  where: { status: 'active', age: { $gte: 21 } },
  orderBy: { createdAt: 'desc' },
  limit: 10
});

// Find one
const admin = await users.findOne({ role: 'admin' });
```

**Update:**
```typescript
// Update by ID
await users.updateById('user-123', {
  status: 'verified',
  verifiedAt: new Date()
});

// Update many
const result = await users.updateMany(
  { status: 'pending' },
  { status: 'active' }
);
console.log(`Updated ${result.modifiedCount} users`);
```

**Delete:**
```typescript
// Delete by ID
await users.deleteById('user-123');

// Delete many
const result = await users.deleteMany({ status: 'inactive' });
console.log(`Deleted ${result.deletedCount} users`);
```

### 2.4 Transactions

```typescript
// ACID transaction
await db.transaction(async (tx) => {
  // Debit from account A
  const accountA = await tx.collection('accounts').findById('acc-a');
  await tx.collection('accounts').updateById('acc-a', {
    balance: accountA.balance - 100
  });

  // Credit to account B
  const accountB = await tx.collection('accounts').findById('acc-b');
  await tx.collection('accounts').updateById('acc-b', {
    balance: accountB.balance + 100
  });

  // Create transaction record
  await tx.collection('transactions').insert({
    from: 'acc-a',
    to: 'acc-b',
    amount: 100,
    timestamp: new Date()
  });
});

// Transaction with isolation level
await db.transaction(
  async (tx) => {
    // ... operations
  },
  { isolationLevel: 'SERIALIZABLE', timeout: 30000 }
);
```

---

## 3. Query Language Usage

### 3.1 LQL (SQL-Like Queries)

```typescript
// Basic SELECT
const users = await db.lql(`
  SELECT id, name, email, created_at
  FROM users
  WHERE status = 'active'
  ORDER BY created_at DESC
  LIMIT 10
`);

// Aggregations
const stats = await db.lql(`
  SELECT
    category,
    COUNT(*) as total,
    AVG(price) as avg_price,
    SUM(quantity) as total_quantity
  FROM products
  GROUP BY category
  HAVING COUNT(*) > 5
  ORDER BY total DESC
`);

// Joins
const ordersWithUsers = await db.lql(`
  SELECT
    o.id as order_id,
    o.total,
    u.name as customer_name,
    u.email as customer_email
  FROM orders o
  INNER JOIN users u ON o.user_id = u.id
  WHERE o.status = 'completed'
    AND o.created_at > '2024-01-01'
`);

// Subqueries
const highValueCustomers = await db.lql(`
  SELECT * FROM users
  WHERE id IN (
    SELECT user_id FROM orders
    GROUP BY user_id
    HAVING SUM(total) > 1000
  )
`);

// Window functions
const rankedSales = await db.lql(`
  SELECT
    salesperson,
    revenue,
    RANK() OVER (ORDER BY revenue DESC) as rank,
    SUM(revenue) OVER () as total_revenue
  FROM sales
`);

// INSERT
await db.lql(`
  INSERT INTO users (name, email, age)
  VALUES ('Alice', 'alice@example.com', 28)
`);

// UPDATE
await db.lql(`
  UPDATE users
  SET status = 'verified', verified_at = NOW()
  WHERE email_verified = true AND status = 'pending'
`);

// DELETE
await db.lql(`
  DELETE FROM sessions
  WHERE expires_at < NOW()
`);
```

### 3.2 NQL (Natural Language Queries)

```typescript
// Find queries
const users = await db.nql('find all users');
const activeUsers = await db.nql('get users where status equals active');
const recentUsers = await db.nql('show first 10 users sorted by created_at descending');

// Aggregations
const count = await db.nql('count all orders where status equals completed');
const avgPrice = await db.nql('find average price from products grouped by category');

// Insert
await db.nql('add to users name "Alice", email "alice@example.com", age 28');

// Update
await db.nql('update users set status to verified where email_verified is true');

// Delete
await db.nql('remove from sessions where expired is true');

// Complex queries
const results = await db.nql(`
  find all products where price is greater than 50
  and category equals electronics
  sorted by rating descending
  limit 20
`);
```

### 3.3 JQL (JSON Query Language)

```typescript
// Find with filters
const users = await db.jql({
  find: 'users',
  filter: {
    age: { $gte: 21, $lte: 65 },
    status: { $in: ['active', 'verified'] }
  },
  projection: { name: 1, email: 1, _id: 1 },
  sort: { createdAt: -1 },
  limit: 10,
  skip: 20
});

// Aggregation pipeline
const analytics = await db.jql({
  aggregate: 'orders',
  pipeline: [
    { $match: { status: 'completed', createdAt: { $gte: '2024-01-01' } } },
    { $group: {
        _id: { month: { $month: '$createdAt' } },
        totalRevenue: { $sum: '$total' },
        orderCount: { $sum: 1 },
        avgOrderValue: { $avg: '$total' }
      }
    },
    { $sort: { '_id.month': 1 } }
  ]
});

// Insert
await db.jql({
  insert: 'users',
  documents: [
    { name: 'Alice', email: 'alice@example.com', age: 28 },
    { name: 'Bob', email: 'bob@example.com', age: 32 }
  ]
});

// Update
await db.jql({
  update: 'users',
  filter: { status: 'pending' },
  update: { $set: { status: 'active', activatedAt: new Date() } }
});

// Delete
await db.jql({
  delete: 'sessions',
  filter: { expiresAt: { $lt: new Date() } }
});
```

### 3.4 Multi-Dialect Queries

```typescript
// InfluxQL
const metrics = await db.query(`
  SELECT mean("cpu_usage")
  FROM "server_metrics"
  WHERE time > now() - 1h
  GROUP BY time(5m), "host"
`, { dialect: 'influxql' });

// PromQL
const alerts = await db.query(`
  rate(http_requests_total[5m]) > 100
`, { dialect: 'promql' });

// Auto-detect dialect
const result = await db.query(userQuery, { dialect: 'auto' });
console.log('Detected dialect:', result.detectedDialect);
```

---

## 4. Indexing & Performance

### 4.1 Creating Indexes

```typescript
// B-Tree index (for range queries)
await users.createIndex({
  name: 'idx_users_age',
  fields: ['age'],
  type: 'btree'
});

// Compound index
await orders.createIndex({
  name: 'idx_orders_user_date',
  fields: ['user_id', 'created_at'],
  type: 'btree'
});

// Unique index
await users.createIndex({
  name: 'idx_users_email',
  fields: ['email'],
  type: 'hash',
  unique: true
});

// Full-text index
await products.createIndex({
  name: 'idx_products_search',
  fields: ['name', 'description'],
  type: 'fulltext',
  options: {
    language: 'english',
    weights: { name: 2, description: 1 }
  }
});

// Sparse index (only index documents with field)
await users.createIndex({
  name: 'idx_users_phone',
  fields: ['phone'],
  type: 'btree',
  sparse: true
});
```

### 4.2 Query Optimization

```typescript
// Explain query plan
const plan = await db.lql(`
  EXPLAIN SELECT * FROM orders
  WHERE user_id = 'user-123'
  AND created_at > '2024-01-01'
`);
console.log('Query plan:', plan);

// Force index usage
const results = await db.lql(`
  SELECT * FROM orders USE INDEX (idx_orders_user_date)
  WHERE user_id = 'user-123'
`);
```

### 4.3 Performance Configuration

```typescript
// Configure storage engine
const db = Database.create('highperf_db', {
  storage: 'file',
  path: './data',
  config: {
    memtable_size: 128 * 1024 * 1024,  // 128 MB
    block_cache_size: 512 * 1024 * 1024, // 512 MB
    compression: 'lz4',
    sync_mode: 'group_commit',
    bloom_bits_per_key: 10
  }
});
```

---

## 5. AI & Vector Search

### 5.1 Setting Up Vector Search

```typescript
// Configure AI service
const db = Database.create('ai_app', {
  ai: {
    provider: 'openai',
    apiKey: process.env.OPENAI_API_KEY,
    model: 'text-embedding-ada-002'
  }
});
await db.open();
```

### 5.2 Indexing Documents for Vector Search

```typescript
// Index single document
await db.ai.index('products', {
  id: 'prod-001',
  text: 'Wireless noise-canceling headphones with 30-hour battery life',
  metadata: {
    category: 'electronics',
    price: 299.99,
    brand: 'AudioTech'
  }
});

// Batch index
await db.ai.indexMany('products', [
  {
    id: 'prod-002',
    text: 'Professional gaming keyboard with RGB lighting',
    metadata: { category: 'electronics', price: 149.99 }
  },
  {
    id: 'prod-003',
    text: 'Ergonomic office chair with lumbar support',
    metadata: { category: 'furniture', price: 399.99 }
  }
]);

// Index from collection
await db.ai.indexCollection('articles', {
  textField: 'content',
  metadataFields: ['author', 'category', 'publishedAt']
});
```

### 5.3 Semantic Search

```typescript
// Basic similarity search
const results = await db.ai.search('products', {
  query: 'comfortable headphones for long flights',
  topK: 10
});

// Search with filters
const filteredResults = await db.ai.search('products', {
  query: 'wireless audio device',
  topK: 5,
  filter: {
    category: 'electronics',
    price: { $lte: 200 }
  }
});

// Search with score threshold
const highQualityResults = await db.ai.search('products', {
  query: 'gaming peripherals',
  topK: 10,
  minScore: 0.8
});

// Hybrid search (vector + keyword)
const hybridResults = await db.ai.search('products', {
  query: 'wireless keyboard',
  topK: 10,
  hybrid: {
    enabled: true,
    keywordWeight: 0.3,
    vectorWeight: 0.7
  }
});
```

### 5.4 PromptQL (Natural Language to Query)

```typescript
import { PromptQLEngine, LLMConfig } from 'tdb-plus';

// Initialize PromptQL
const promptql = new PromptQLEngine({
  db: db,
  llm: {
    provider: 'openai',
    model: 'gpt-4',
    apiKey: process.env.OPENAI_API_KEY
  }
});

// Natural language query
const result = await promptql.query(
  "Find all customers who spent more than $500 last month"
);
console.log('Generated query:', result.query);
console.log('Results:', result.data);

// Multi-step reasoning
const analysis = await promptql.query(
  "Compare this month's sales with last month, " +
  "identify top performing products, " +
  "and suggest inventory adjustments"
);

// Conversational follow-up
await promptql.query("Now filter those to just electronics");
await promptql.query("What's the average price?");
```

### 5.5 Custom Embeddings

```typescript
// Use custom embedding model
const db = Database.create('custom_ai', {
  ai: {
    provider: 'custom',
    embeddingFn: async (text) => {
      // Your custom embedding logic
      const response = await myEmbeddingService.embed(text);
      return response.vector;
    },
    dimensions: 768
  }
});

// Use local model
const db = Database.create('local_ai', {
  ai: {
    provider: 'local',
    modelPath: './models/all-MiniLM-L6-v2'
  }
});
```

---

## 6. Time-Series Operations

### 6.1 Creating Time-Series Tables

```sql
-- Using LQL
CREATE TABLE sensor_data (
  ts TIMESTAMP NOT NULL,
  device_id VARCHAR(64),
  temperature FLOAT,
  humidity FLOAT,
  pressure FLOAT
) TAGS (
  location VARCHAR(64),
  device_type VARCHAR(32)
);

-- Create super table (TDengine-compatible)
CREATE STABLE metrics (
  ts TIMESTAMP,
  value DOUBLE,
  quality INT
) TAGS (
  metric_name VARCHAR(128),
  host VARCHAR(64),
  region VARCHAR(32)
);
```

### 6.2 Inserting Time-Series Data

```typescript
// Single insert
await db.lql(`
  INSERT INTO sensor_data (ts, device_id, temperature, humidity)
  VALUES (NOW, 'device-001', 23.5, 65.2)
`);

// Batch insert
await db.lql(`
  INSERT INTO sensor_data (ts, device_id, temperature, humidity) VALUES
    ('2024-01-15 10:00:00', 'device-001', 23.5, 65.2),
    ('2024-01-15 10:01:00', 'device-001', 23.6, 65.1),
    ('2024-01-15 10:02:00', 'device-001', 23.7, 65.0)
`);

// Schemaless ingestion (InfluxDB line protocol)
await db.write(`
  sensor_data,location=building_a,device_type=thermometer temperature=23.5,humidity=65.2 1705312800000000000
  sensor_data,location=building_a,device_type=thermometer temperature=23.6,humidity=65.1 1705312860000000000
`, { format: 'influx' });
```

### 6.3 Time-Series Queries

```sql
-- Basic time range query
SELECT ts, temperature, humidity
FROM sensor_data
WHERE device_id = 'device-001'
  AND ts >= '2024-01-15 00:00:00'
  AND ts < '2024-01-16 00:00:00'
ORDER BY ts;

-- Downsampling with INTERVAL
SELECT
  INTERVAL(ts, '1h') as hour,
  AVG(temperature) as avg_temp,
  MAX(temperature) as max_temp,
  MIN(temperature) as min_temp
FROM sensor_data
WHERE ts >= NOW - INTERVAL '24 hours'
GROUP BY INTERVAL(ts, '1h');

-- Sliding windows
SELECT
  ts,
  AVG(temperature) OVER (ORDER BY ts ROWS BETWEEN 5 PRECEDING AND CURRENT ROW) as moving_avg
FROM sensor_data
WHERE device_id = 'device-001';

-- TDengine-compatible window functions
SELECT
  _wstart as window_start,
  _wend as window_end,
  AVG(temperature) as avg_temp,
  FIRST(temperature) as first_temp,
  LAST(temperature) as last_temp,
  SPREAD(temperature) as temp_range
FROM sensor_data
WHERE ts >= '2024-01-15'
INTERVAL(1h) SLIDING(15m);

-- Session windows
SELECT
  _wstart, _wend,
  COUNT(*) as events,
  SUM(value) as total
FROM user_events
SESSION(ts, 30m)
WHERE user_id = 'user-123';

-- State windows
SELECT
  _wstart, _wend,
  status,
  ELAPSED(ts) as duration
FROM machine_status
STATE_WINDOW(status)
WHERE machine_id = 'machine-001';
```

### 6.4 Time-Series Aggregations

```sql
-- Time-weighted average
SELECT TWA(temperature) as weighted_avg
FROM sensor_data
WHERE ts BETWEEN '2024-01-15' AND '2024-01-16';

-- Approximate percentiles
SELECT
  APERCENTILE(response_time, 50) as p50,
  APERCENTILE(response_time, 95) as p95,
  APERCENTILE(response_time, 99) as p99
FROM api_metrics
WHERE ts >= NOW - INTERVAL '1 hour';

-- Rate calculation
SELECT
  DERIVATIVE(counter_value, '1s') as rate_per_second
FROM counters
WHERE ts >= NOW - INTERVAL '5 minutes';

-- Gap filling with interpolation
SELECT
  ts,
  INTERP(temperature) as interpolated_temp
FROM sensor_data
WHERE ts BETWEEN '2024-01-15 10:00:00' AND '2024-01-15 11:00:00'
RANGE('2024-01-15 10:00:00', '2024-01-15 11:00:00')
EVERY(1m)
FILL(LINEAR);
```

---

## 7. GraphQL & REST APIs

### 7.1 Auto-Generated GraphQL

```graphql
# Query users with filtering
query GetUsers {
  users(
    where: { status: { _eq: "active" }, age: { _gte: 21 } }
    orderBy: { createdAt: DESC }
    limit: 10
  ) {
    id
    name
    email
    createdAt
  }
}

# Get user by ID with relations
query GetUserWithOrders($userId: ID!) {
  user(id: $userId) {
    id
    name
    email
    orders {
      id
      total
      status
      items {
        productId
        quantity
        price
      }
    }
  }
}

# Aggregations
query GetOrderStats {
  ordersAggregate(where: { status: { _eq: "completed" } }) {
    count
    sum {
      total
    }
    avg {
      total
    }
  }
}

# Insert mutation
mutation CreateUser($input: UserInput!) {
  insertUser(object: $input) {
    id
    name
    email
  }
}

# Update mutation
mutation UpdateUser($id: ID!, $changes: UserUpdateInput!) {
  updateUser(id: $id, set: $changes) {
    id
    name
    status
    updatedAt
  }
}

# Subscription
subscription OnNewOrder {
  orderCreated {
    id
    userId
    total
    createdAt
  }
}
```

### 7.2 GraphQL Client Usage

```typescript
// Using fetch
const response = await fetch('http://localhost:8080/v1/graphql', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer YOUR_JWT_TOKEN'
  },
  body: JSON.stringify({
    query: `
      query GetUsers($status: String!) {
        users(where: { status: { _eq: $status } }) {
          id
          name
          email
        }
      }
    `,
    variables: { status: 'active' }
  })
});

const { data, errors } = await response.json();
```

### 7.3 REST API Usage

```bash
# List collection
curl -X GET "http://localhost:8080/api/v1/users?status=active&limit=10" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Get by ID
curl -X GET "http://localhost:8080/api/v1/users/user-123" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Create
curl -X POST "http://localhost:8080/api/v1/users" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"name": "Alice", "email": "alice@example.com", "age": 28}'

# Update
curl -X PATCH "http://localhost:8080/api/v1/users/user-123" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"status": "verified"}'

# Delete
curl -X DELETE "http://localhost:8080/api/v1/users/user-123" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Execute query
curl -X POST "http://localhost:8080/api/v1/query" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"query": "SELECT * FROM users WHERE age > 21 LIMIT 10"}'
```

### 7.4 Event Triggers & Webhooks

```typescript
// Create event trigger
await db.createTrigger({
  name: 'new_order_notification',
  table: 'orders',
  events: ['INSERT'],
  webhook: {
    url: 'https://api.example.com/webhooks/new-order',
    headers: {
      'Authorization': 'Bearer ${WEBHOOK_SECRET}'
    },
    retryConfig: {
      maxRetries: 3,
      retryInterval: 10
    }
  }
});

// Trigger with transformation
await db.createTrigger({
  name: 'user_analytics',
  table: 'users',
  events: ['INSERT', 'UPDATE'],
  webhook: {
    url: 'https://analytics.example.com/events',
    transform: {
      event_type: '$.operation',
      user_id: '$.new.id',
      timestamp: '$.timestamp'
    }
  }
});

// Kafka/Redpanda trigger
await db.createTrigger({
  name: 'order_stream',
  table: 'orders',
  events: ['INSERT', 'UPDATE', 'DELETE'],
  kafka: {
    bootstrapServers: 'kafka:9092',
    topic: 'order-events',
    keyField: 'id'
  }
});
```

---

## 8. Security Configuration

### 8.1 Authentication Setup

```typescript
// Enable JWT authentication
const config = {
  security: {
    requireAuth: true,
    jwt: {
      secret: process.env.JWT_SECRET,
      algorithm: 'HS256',
      expiresIn: '24h'
    }
  }
};

// Generate JWT token
import jwt from 'jsonwebtoken';

const token = jwt.sign(
  {
    userId: 'user-123',
    role: 'admin',
    permissions: ['read', 'write', 'admin']
  },
  process.env.JWT_SECRET,
  { expiresIn: '24h' }
);
```

### 8.2 Role-Based Access Control (RBAC)

```yaml
# rbac-config.yaml
roles:
  admin:
    description: "Full system access"
    permissions:
      - "*"

  developer:
    description: "Development access"
    permissions:
      - "read:*"
      - "write:development_*"
      - "write:staging_*"
      - "execute:queries"

  analyst:
    description: "Read-only analytics access"
    permissions:
      - "read:analytics_*"
      - "read:reports_*"
      - "execute:select"

  app_service:
    description: "Application service account"
    permissions:
      - "read:users"
      - "write:users"
      - "read:orders"
      - "write:orders"
      - "execute:queries"

users:
  - username: alice
    role: admin

  - username: bob
    role: developer

  - username: analytics_service
    role: analyst
    api_key: "${ANALYTICS_API_KEY}"
```

### 8.3 Row-Level Security

```sql
-- Enable row-level security on table
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;

-- Create policy for users to see only their orders
CREATE POLICY user_orders ON orders
  FOR ALL
  USING (user_id = current_user_id());

-- Admin can see all
CREATE POLICY admin_all ON orders
  FOR ALL
  TO admin
  USING (true);
```

### 8.4 Encryption Configuration

```toml
# config.toml
[security.encryption]
# TLS for connections
tls_enabled = true
tls_cert_file = "/etc/lumadb/certs/server.crt"
tls_key_file = "/etc/lumadb/certs/server.key"
tls_ca_file = "/etc/lumadb/certs/ca.crt"

# At-rest encryption
at_rest_enabled = true
at_rest_algorithm = "AES-256-GCM"
key_rotation_days = 90

# Field-level encryption
field_encryption_enabled = true
encrypted_fields = ["ssn", "credit_card", "password"]
```

---

## 9. Cluster Deployment

### 9.1 Three-Node Cluster Setup

```yaml
# docker-compose-cluster.yml
version: '3.8'

services:
  lumadb-1:
    image: lumadb/lumadb:latest
    environment:
      LUMADB_NODE_ID: node-1
      LUMADB_RAFT_ADDRESS: lumadb-1:10000
      LUMADB_CLUSTER_PEERS: lumadb-2:10000,lumadb-3:10000
      LUMADB_REPLICATION_FACTOR: 3
    ports:
      - "8081:8080"
    volumes:
      - node1-data:/var/lib/lumadb

  lumadb-2:
    image: lumadb/lumadb:latest
    environment:
      LUMADB_NODE_ID: node-2
      LUMADB_RAFT_ADDRESS: lumadb-2:10000
      LUMADB_CLUSTER_PEERS: lumadb-1:10000,lumadb-3:10000
      LUMADB_REPLICATION_FACTOR: 3
    ports:
      - "8082:8080"
    volumes:
      - node2-data:/var/lib/lumadb

  lumadb-3:
    image: lumadb/lumadb:latest
    environment:
      LUMADB_NODE_ID: node-3
      LUMADB_RAFT_ADDRESS: lumadb-3:10000
      LUMADB_CLUSTER_PEERS: lumadb-1:10000,lumadb-2:10000
      LUMADB_REPLICATION_FACTOR: 3
    ports:
      - "8083:8080"
    volumes:
      - node3-data:/var/lib/lumadb

  load-balancer:
    image: haproxy:latest
    ports:
      - "8080:8080"
    volumes:
      - ./haproxy.cfg:/usr/local/etc/haproxy/haproxy.cfg

volumes:
  node1-data:
  node2-data:
  node3-data:
```

### 9.2 Cluster Client Configuration

```typescript
import { createClient } from 'tdb-plus/client';

const client = createClient({
  nodes: [
    'lumadb-1:8080',
    'lumadb-2:8080',
    'lumadb-3:8080'
  ],
  pool: {
    minConnections: 10,
    maxConnections: 100,
    idleTimeout: 30000
  },
  loadBalancing: 'round-robin',
  retryPolicy: {
    maxRetries: 3,
    backoffMultiplier: 2,
    initialDelay: 100
  },
  healthCheck: {
    enabled: true,
    interval: 5000
  }
});

// Client automatically handles failover
const result = await client.query('SELECT * FROM users');
```

### 9.3 Cluster Management

```bash
# Check cluster status
curl http://localhost:8080/cluster/status

# Add new node
curl -X POST http://localhost:8080/cluster/nodes \
  -H "Content-Type: application/json" \
  -d '{"nodeId": "node-4", "address": "lumadb-4:10000"}'

# Remove node
curl -X DELETE http://localhost:8080/cluster/nodes/node-4

# Rebalance shards
curl -X POST http://localhost:8080/cluster/rebalance

# Force leader election
curl -X POST http://localhost:8080/cluster/election
```

---

## 10. Monitoring & Observability

### 10.1 Prometheus Setup

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'lumadb'
    static_configs:
      - targets:
          - 'lumadb-1:9090'
          - 'lumadb-2:9090'
          - 'lumadb-3:9090'
    metrics_path: /metrics
```

### 10.2 Key Metrics to Monitor

```promql
# Query latency
histogram_quantile(0.99, sum(rate(lumadb_query_duration_seconds_bucket[5m])) by (le))

# Throughput
sum(rate(lumadb_query_total[5m])) by (type)

# Active connections
lumadb_connections_active

# Storage usage
lumadb_storage_bytes_total

# Replication lag
lumadb_replication_lag_seconds

# Cache hit rate
sum(rate(lumadb_cache_hits_total[5m])) / sum(rate(lumadb_cache_requests_total[5m]))
```

### 10.3 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "LumaDB Overview",
    "panels": [
      {
        "title": "Query Latency (p99)",
        "type": "graph",
        "targets": [{
          "expr": "histogram_quantile(0.99, sum(rate(lumadb_query_duration_seconds_bucket[5m])) by (le))"
        }]
      },
      {
        "title": "Queries per Second",
        "type": "graph",
        "targets": [{
          "expr": "sum(rate(lumadb_query_total[1m]))"
        }]
      },
      {
        "title": "Storage Usage",
        "type": "gauge",
        "targets": [{
          "expr": "sum(lumadb_storage_bytes_total)"
        }]
      }
    ]
  }
}
```

### 10.4 Alerting Rules

```yaml
# alerts.yml
groups:
  - name: lumadb
    rules:
      - alert: HighQueryLatency
        expr: histogram_quantile(0.99, sum(rate(lumadb_query_duration_seconds_bucket[5m])) by (le)) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High query latency detected"

      - alert: ClusterNodeDown
        expr: up{job="lumadb"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "LumaDB node is down"

      - alert: HighReplicationLag
        expr: lumadb_replication_lag_seconds > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High replication lag"
```

---

## 11. Backup & Recovery

### 11.1 Backup Operations

```bash
# Full backup to local directory
lumadb backup \
  --destination /backups/$(date +%Y%m%d) \
  --type full

# Backup to S3
lumadb backup \
  --destination s3://my-bucket/backups/$(date +%Y%m%d) \
  --type full \
  --compression zstd

# Incremental backup
lumadb backup \
  --destination s3://my-bucket/backups/incremental \
  --type incremental \
  --since-backup /backups/20240115

# Backup specific collections
lumadb backup \
  --destination /backups/users \
  --collections users,orders
```

### 11.2 Restore Operations

```bash
# Full restore
lumadb restore \
  --source /backups/20240115 \
  --target-db restored_db

# Restore from S3
lumadb restore \
  --source s3://my-bucket/backups/20240115 \
  --target-db production

# Point-in-time recovery
lumadb restore \
  --source /backups/20240115 \
  --point-in-time "2024-01-15T14:30:00Z"

# Restore specific collection
lumadb restore \
  --source /backups/20240115 \
  --collections users \
  --target-db production
```

### 11.3 Automated Backup Schedule

```yaml
# backup-cronjob.yaml (Kubernetes)
apiVersion: batch/v1
kind: CronJob
metadata:
  name: lumadb-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: backup
              image: lumadb/tools:latest
              command:
                - lumadb
                - backup
                - --destination
                - s3://backups/$(date +%Y%m%d)
                - --type
                - full
              env:
                - name: AWS_ACCESS_KEY_ID
                  valueFrom:
                    secretKeyRef:
                      name: aws-credentials
                      key: access-key
          restartPolicy: OnFailure
```

---

## 12. Migration Guides

### 12.1 PostgreSQL to LumaDB

```bash
# Export from PostgreSQL
pg_dump -h source-host -U user -d database --format=plain > export.sql

# Transform and import (LumaDB accepts PostgreSQL SQL)
lumadb import \
  --source export.sql \
  --format postgresql \
  --target-db my_database

# Or use live replication
lumadb migrate \
  --source "postgresql://user:pass@source:5432/db" \
  --mode live \
  --parallel 4
```

### 12.2 MongoDB to LumaDB

```bash
# Export from MongoDB
mongodump --uri="mongodb://source:27017/database" --out=/backup

# Import to LumaDB
lumadb import \
  --source /backup/database \
  --format mongodb \
  --target-db my_database
```

### 12.3 InfluxDB to LumaDB

```bash
# Export from InfluxDB
influx backup /backup -t $INFLUX_TOKEN

# Import to LumaDB (preserves time-series structure)
lumadb import \
  --source /backup \
  --format influxdb \
  --target-db metrics_db
```

### 12.4 Live Migration with Zero Downtime

```bash
# Start shadow replication
lumadb migrate \
  --source "postgresql://old-db:5432/app" \
  --mode shadow \
  --verify-reads

# Monitor replication lag
watch lumadb migrate status

# When lag is near zero, cutover
lumadb migrate cutover --confirm

# Rollback if needed
lumadb migrate rollback
```

---

*Document Version: 3.0.0*
*Last Updated: December 2024*
