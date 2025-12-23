# LumaDB API & SDK Reference Manual

## Complete API Documentation

**Version:** 3.0.0 | **Last Updated:** December 2024

---

## Table of Contents

1. [REST API Reference](#1-rest-api-reference)
2. [GraphQL API Reference](#2-graphql-api-reference)
3. [TypeScript SDK Reference](#3-typescript-sdk-reference)
4. [Python SDK Reference](#4-python-sdk-reference)
5. [Protocol Reference](#5-protocol-reference)
6. [Error Handling](#6-error-handling)

---

## 1. REST API Reference

### 1.1 Authentication

**JWT Token Authentication:**
```bash
# Obtain token
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-password"}'

# Response
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_at": "2024-01-16T12:00:00Z",
  "user": {
    "id": "user-123",
    "username": "admin",
    "role": "admin"
  }
}

# Use token in requests
curl http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

**API Key Authentication:**
```bash
curl http://localhost:8080/api/v1/users \
  -H "X-API-Key: your-api-key"
```

### 1.2 Collection Endpoints

#### List Collections
```
GET /api/v1/collections
```

**Response:**
```json
{
  "collections": [
    {
      "name": "users",
      "document_count": 15420,
      "size_bytes": 45678912,
      "indexes": ["_id", "email", "created_at"],
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "total": 5
}
```

#### Get Collection Info
```
GET /api/v1/collections/{name}
```

#### Create Collection
```
POST /api/v1/collections
```

**Request:**
```json
{
  "name": "products",
  "schema": {
    "name": { "type": "string", "required": true },
    "price": { "type": "number", "min": 0 },
    "category": { "type": "string" }
  },
  "indexes": [
    { "fields": ["category"], "type": "btree" }
  ]
}
```

#### Delete Collection
```
DELETE /api/v1/collections/{name}
```

### 1.3 Document Endpoints

#### List Documents
```
GET /api/v1/{collection}
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `filter` | JSON | Filter conditions |
| `sort` | string | Sort field and direction |
| `limit` | integer | Max documents (default: 20) |
| `offset` | integer | Skip documents |
| `fields` | string | Comma-separated field list |

**Example:**
```bash
curl "http://localhost:8080/api/v1/users?filter={\"status\":\"active\"}&sort=-created_at&limit=10"
```

**Response:**
```json
{
  "data": [
    {
      "_id": "user-123",
      "name": "Alice",
      "email": "alice@example.com",
      "status": "active",
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "pagination": {
    "total": 150,
    "limit": 10,
    "offset": 0,
    "has_more": true
  }
}
```

#### Get Document by ID
```
GET /api/v1/{collection}/{id}
```

#### Create Document
```
POST /api/v1/{collection}
```

**Request:**
```json
{
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 28,
  "tags": ["developer", "team-a"]
}
```

**Response:**
```json
{
  "_id": "user-abc123",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "age": 28,
  "tags": ["developer", "team-a"],
  "_rev": 1,
  "_createdAt": "2024-01-15T10:00:00Z",
  "_updatedAt": "2024-01-15T10:00:00Z"
}
```

#### Bulk Create
```
POST /api/v1/{collection}/bulk
```

**Request:**
```json
{
  "documents": [
    { "name": "Bob", "email": "bob@example.com" },
    { "name": "Carol", "email": "carol@example.com" }
  ]
}
```

#### Update Document
```
PUT /api/v1/{collection}/{id}
```

**Full replacement of document.**

#### Partial Update
```
PATCH /api/v1/{collection}/{id}
```

**Request:**
```json
{
  "status": "verified",
  "verified_at": "2024-01-15T12:00:00Z"
}
```

#### Delete Document
```
DELETE /api/v1/{collection}/{id}
```

### 1.4 Query Endpoint

#### Execute Query
```
POST /api/v1/query
```

**LQL Query:**
```json
{
  "query": "SELECT * FROM users WHERE age > 21 ORDER BY name LIMIT 10",
  "params": {}
}
```

**Parameterized Query:**
```json
{
  "query": "SELECT * FROM users WHERE status = $1 AND age > $2",
  "params": ["active", 21]
}
```

**Response:**
```json
{
  "columns": ["_id", "name", "email", "age", "status"],
  "rows": [
    ["user-123", "Alice", "alice@example.com", 28, "active"],
    ["user-456", "Bob", "bob@example.com", 32, "active"]
  ],
  "row_count": 2,
  "execution_time_ms": 12
}
```

### 1.5 Index Endpoints

#### List Indexes
```
GET /api/v1/{collection}/indexes
```

#### Create Index
```
POST /api/v1/{collection}/indexes
```

**Request:**
```json
{
  "name": "idx_email",
  "fields": ["email"],
  "type": "hash",
  "unique": true
}
```

#### Delete Index
```
DELETE /api/v1/{collection}/indexes/{name}
```

### 1.6 Admin Endpoints

#### Health Check
```
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "3.0.0",
  "uptime_seconds": 86400,
  "checks": {
    "storage": "ok",
    "cluster": "ok",
    "memory": "ok"
  }
}
```

#### Metrics (Prometheus)
```
GET /metrics
```

#### Statistics
```
GET /api/v1/stats
```

**Response:**
```json
{
  "collections": 12,
  "documents": 1542890,
  "storage_bytes": 4567891234,
  "queries_per_second": 4521,
  "cache_hit_rate": 0.94,
  "connections": {
    "active": 45,
    "idle": 155,
    "total": 200
  }
}
```

---

## 2. GraphQL API Reference

### 2.1 Endpoint

```
POST /v1/graphql
```

### 2.2 Schema Overview

```graphql
# Auto-generated types for each collection
type User {
  _id: ID!
  name: String!
  email: String!
  age: Int
  status: String
  createdAt: DateTime!
  updatedAt: DateTime!

  # Relations (if defined)
  orders: [Order!]!
}

type Query {
  # Single document
  user(id: ID!): User

  # List with filtering
  users(
    where: UserFilter
    orderBy: [UserOrderBy!]
    limit: Int
    offset: Int
  ): [User!]!

  # Aggregations
  usersAggregate(where: UserFilter): UserAggregate!
}

type Mutation {
  # Create
  insertUser(object: UserInput!): User!
  insertUsers(objects: [UserInput!]!): [User!]!

  # Update
  updateUser(id: ID!, set: UserUpdateInput!): User
  updateUsers(where: UserFilter!, set: UserUpdateInput!): UpdateResult!

  # Delete
  deleteUser(id: ID!): User
  deleteUsers(where: UserFilter!): DeleteResult!
}

type Subscription {
  userCreated: User!
  userUpdated(id: ID): User!
  userDeleted: ID!
}

# Filter types
input UserFilter {
  _id: IDFilter
  name: StringFilter
  email: StringFilter
  age: IntFilter
  status: StringFilter
  _and: [UserFilter!]
  _or: [UserFilter!]
  _not: UserFilter
}

input StringFilter {
  _eq: String
  _neq: String
  _in: [String!]
  _nin: [String!]
  _like: String
  _ilike: String
  _regex: String
}

input IntFilter {
  _eq: Int
  _neq: Int
  _gt: Int
  _gte: Int
  _lt: Int
  _lte: Int
  _in: [Int!]
}
```

### 2.3 Query Examples

**Basic Query:**
```graphql
query GetActiveUsers {
  users(
    where: { status: { _eq: "active" } }
    orderBy: [{ createdAt: DESC }]
    limit: 10
  ) {
    _id
    name
    email
    createdAt
  }
}
```

**Query with Relations:**
```graphql
query GetUserWithOrders($userId: ID!) {
  user(id: $userId) {
    _id
    name
    email
    orders(where: { status: { _eq: "completed" } }) {
      _id
      total
      items {
        productName
        quantity
        price
      }
    }
  }
}
```

**Aggregation:**
```graphql
query GetOrderStats {
  ordersAggregate(
    where: { createdAt: { _gte: "2024-01-01" } }
  ) {
    count
    sum { total }
    avg { total }
    min { total }
    max { total }
  }
}
```

**Mutation:**
```graphql
mutation CreateUser($input: UserInput!) {
  insertUser(object: $input) {
    _id
    name
    email
    createdAt
  }
}

# Variables
{
  "input": {
    "name": "Alice Johnson",
    "email": "alice@example.com",
    "age": 28
  }
}
```

**Subscription:**
```graphql
subscription OnOrderCreated {
  orderCreated {
    _id
    userId
    total
    status
    createdAt
  }
}
```

### 2.4 WebSocket Subscriptions

```javascript
// Using graphql-ws
import { createClient } from 'graphql-ws';

const client = createClient({
  url: 'ws://localhost:8080/v1/graphql',
  connectionParams: {
    authorization: 'Bearer YOUR_JWT_TOKEN'
  }
});

const unsubscribe = client.subscribe(
  {
    query: `subscription { orderCreated { _id total } }`
  },
  {
    next: (data) => console.log('New order:', data),
    error: (err) => console.error('Error:', err),
    complete: () => console.log('Subscription complete')
  }
);
```

---

## 3. TypeScript SDK Reference

### 3.1 Installation

```bash
npm install tdb-plus
# or
yarn add tdb-plus
```

### 3.2 Database Class

```typescript
import { Database, DatabaseConfig } from 'tdb-plus';

// Create database instance
const db = Database.create('my_app', {
  storage: 'file',
  path: './data',
  cache: {
    enabled: true,
    maxSize: 1000
  }
});

// Open connection
await db.open();

// Close connection
await db.close();
```

**Configuration Options:**
```typescript
interface DatabaseConfig {
  storage: 'memory' | 'file';
  path?: string;
  cache?: {
    enabled: boolean;
    maxSize: number;
    ttl?: number;
  };
  ai?: {
    provider: 'openai' | 'anthropic' | 'local';
    apiKey?: string;
    model?: string;
  };
}
```

### 3.3 Collection Class

```typescript
// Get collection
const users = db.collection('users');

// With TypeScript generics
interface User {
  name: string;
  email: string;
  age: number;
  status: 'active' | 'inactive';
}

const users = db.collection<User>('users');
```

**Insert Operations:**
```typescript
// Single insert
const user = await users.insert({
  name: 'Alice',
  email: 'alice@example.com',
  age: 28
});

// Batch insert
const newUsers = await users.insertMany([
  { name: 'Bob', email: 'bob@example.com', age: 32 },
  { name: 'Carol', email: 'carol@example.com', age: 25 }
]);
```

**Find Operations:**
```typescript
// Find by ID
const user = await users.findById('user-123');

// Find one
const admin = await users.findOne({ role: 'admin' });

// Find many with options
const activeUsers = await users.find({
  where: {
    status: 'active',
    age: { $gte: 21 }
  },
  orderBy: { createdAt: 'desc' },
  limit: 10,
  offset: 0
});

// Count
const count = await users.count({ status: 'active' });
```

**Update Operations:**
```typescript
// Update by ID
await users.updateById('user-123', {
  status: 'verified'
});

// Update many
const result = await users.updateMany(
  { status: 'pending' },
  { status: 'active' }
);
console.log(`Updated ${result.modifiedCount} documents`);
```

**Delete Operations:**
```typescript
// Delete by ID
await users.deleteById('user-123');

// Delete many
const result = await users.deleteMany({ status: 'inactive' });
```

### 3.4 Query Methods

```typescript
// LQL (SQL-like)
const results = await db.lql(`
  SELECT * FROM users
  WHERE age > 21
  ORDER BY name
  LIMIT 10
`);

// NQL (Natural Language)
const results = await db.nql('find all active users sorted by name');

// JQL (JSON Query)
const results = await db.jql({
  find: 'users',
  filter: { age: { $gt: 21 } },
  sort: { name: 1 },
  limit: 10
});
```

### 3.5 Transaction API

```typescript
await db.transaction(async (tx) => {
  // All operations use transaction context
  const account1 = await tx.collection('accounts').findById('acc-1');
  const account2 = await tx.collection('accounts').findById('acc-2');

  await tx.collection('accounts').updateById('acc-1', {
    balance: account1.balance - 100
  });

  await tx.collection('accounts').updateById('acc-2', {
    balance: account2.balance + 100
  });

  await tx.collection('transfers').insert({
    from: 'acc-1',
    to: 'acc-2',
    amount: 100,
    timestamp: new Date()
  });
});

// With options
await db.transaction(
  async (tx) => { /* ... */ },
  {
    isolationLevel: 'SERIALIZABLE',
    timeout: 30000
  }
);
```

### 3.6 Index API

```typescript
// Create index
await users.createIndex({
  name: 'idx_email',
  fields: ['email'],
  type: 'hash',
  unique: true
});

// Create compound index
await orders.createIndex({
  name: 'idx_user_date',
  fields: ['user_id', 'created_at'],
  type: 'btree'
});

// List indexes
const indexes = await users.getIndexes();

// Drop index
await users.dropIndex('idx_email');
```

### 3.7 AI Features

```typescript
// Configure AI
const db = Database.create('ai_app', {
  ai: {
    provider: 'openai',
    apiKey: process.env.OPENAI_API_KEY,
    model: 'text-embedding-ada-002'
  }
});

// Index for vector search
await db.ai.index('products', {
  id: 'prod-1',
  text: 'Wireless bluetooth headphones',
  metadata: { category: 'electronics', price: 99 }
});

// Semantic search
const results = await db.ai.search('products', {
  query: 'wireless audio device',
  topK: 10,
  filter: { category: 'electronics' }
});

// Natural language to query
const query = await db.ai.translate(
  'show me orders over $100 from last week'
);
```

### 3.8 Client for Remote Connection

```typescript
import { createClient } from 'tdb-plus/client';

const client = createClient({
  url: 'http://localhost:8080',
  auth: {
    type: 'jwt',
    token: 'your-jwt-token'
  },
  pool: {
    maxConnections: 10,
    idleTimeout: 30000
  }
});

// Use same API as local database
const users = await client.lql('SELECT * FROM users LIMIT 10');
```

---

## 4. Python SDK Reference

### 4.1 Installation

```bash
pip install lumadb
```

### 4.2 Connection

```python
from lumadb import LumaDB, Config

# Connect to LumaDB
db = LumaDB(
    host="localhost",
    port=8080,
    auth_token="your-jwt-token"
)

# Or using config
config = Config(
    host="localhost",
    port=8080,
    ssl=True,
    pool_size=10
)
db = LumaDB(config)
```

### 4.3 Query Operations

```python
# Execute LQL query
results = db.query("""
    SELECT * FROM users
    WHERE status = 'active'
    ORDER BY created_at DESC
    LIMIT 10
""")

for row in results:
    print(f"{row['name']}: {row['email']}")

# Parameterized query
results = db.query(
    "SELECT * FROM users WHERE age > $1 AND status = $2",
    params=[21, 'active']
)

# Get single result
user = db.query_one("SELECT * FROM users WHERE id = $1", ['user-123'])
```

### 4.4 Collection Operations

```python
# Get collection
users = db.collection('users')

# Insert
user = users.insert({
    'name': 'Alice',
    'email': 'alice@example.com',
    'age': 28
})

# Find
active_users = users.find(
    where={'status': 'active'},
    order_by=[('-created_at',)],
    limit=10
)

# Update
users.update_by_id('user-123', {'status': 'verified'})

# Delete
users.delete_by_id('user-123')
```

### 4.5 Async Support

```python
import asyncio
from lumadb import AsyncLumaDB

async def main():
    db = AsyncLumaDB(host="localhost", port=8080)

    # Async query
    results = await db.query("SELECT * FROM users LIMIT 10")

    # Async collection operations
    users = db.collection('users')
    user = await users.insert({'name': 'Alice', 'email': 'alice@example.com'})

    await db.close()

asyncio.run(main())
```

### 4.6 AI/Vector Operations

```python
from lumadb.ai import VectorSearch

# Initialize vector search
vector = VectorSearch(db, model='text-embedding-ada-002')

# Index documents
vector.index('products', [
    {'id': 'p1', 'text': 'Wireless headphones', 'metadata': {'price': 99}},
    {'id': 'p2', 'text': 'Gaming keyboard', 'metadata': {'price': 149}}
])

# Search
results = vector.search('products', 'audio device', top_k=5)
```

---

## 5. Protocol Reference

### 5.1 PostgreSQL Wire Protocol

**Connection:**
```
Port: 5432
Protocol: PostgreSQL v3 wire protocol
Authentication: MD5, SCRAM-SHA-256
```

**Compatible Clients:**
- psql
- pgAdmin
- DBeaver
- Any JDBC/ODBC driver
- psycopg2 (Python)
- pg (Node.js)
- Npgsql (.NET)

**Example:**
```bash
psql -h localhost -p 5432 -U admin -d lumadb
```

### 5.2 MySQL Protocol

**Connection:**
```
Port: 3306
Protocol: MySQL wire protocol
Authentication: mysql_native_password
```

**Example:**
```bash
mysql -h localhost -P 3306 -u admin -p lumadb
```

### 5.3 MongoDB Protocol

**Connection:**
```
Port: 27017
Protocol: MongoDB wire protocol (OP_MSG)
```

**Example:**
```bash
mongosh "mongodb://localhost:27017/lumadb"
```

### 5.4 Redis Protocol

**Connection:**
```
Port: 6379
Protocol: RESP (Redis Serialization Protocol)
```

**Supported Commands:**
```
GET, SET, DEL, EXISTS
HGET, HSET, HDEL, HGETALL
LPUSH, RPUSH, LPOP, RPOP, LRANGE
SADD, SREM, SMEMBERS, SISMEMBER
ZADD, ZREM, ZRANGE, ZSCORE
KEYS, SCAN, TTL, EXPIRE
```

### 5.5 InfluxDB Protocol

**Connection:**
```
Port: 8086
Protocol: InfluxDB HTTP API / Line Protocol
```

**Write (Line Protocol):**
```
POST /write?db=metrics

cpu,host=server01 value=0.64 1609459200000000000
memory,host=server01 used=8234567890 1609459200000000000
```

**Query (InfluxQL):**
```
GET /query?db=metrics&q=SELECT mean(value) FROM cpu WHERE time > now() - 1h GROUP BY time(5m)
```

### 5.6 Prometheus Protocol

**Scrape Endpoint:**
```
GET /metrics

# HELP lumadb_query_duration_seconds Query execution time
# TYPE lumadb_query_duration_seconds histogram
lumadb_query_duration_seconds_bucket{le="0.001"} 1234
lumadb_query_duration_seconds_bucket{le="0.01"} 5678
```

**Remote Write:**
```
POST /api/v1/write
Content-Type: application/x-protobuf
X-Prometheus-Remote-Write-Version: 0.1.0
```

**Remote Read:**
```
POST /api/v1/read
```

---

## 6. Error Handling

### 6.1 Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid document: missing required field 'email'",
    "details": {
      "field": "email",
      "constraint": "required"
    },
    "request_id": "req-abc123"
  }
}
```

### 6.2 Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid input data |
| `CONFLICT` | 409 | Resource conflict (duplicate) |
| `QUERY_ERROR` | 400 | Invalid query syntax |
| `TIMEOUT` | 408 | Request timeout |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `UNAVAILABLE` | 503 | Service unavailable |

### 6.3 SDK Error Handling

**TypeScript:**
```typescript
import { LumaDBError, ValidationError, NotFoundError } from 'tdb-plus';

try {
  await users.insert({ name: 'Alice' }); // missing email
} catch (error) {
  if (error instanceof ValidationError) {
    console.log('Validation failed:', error.details);
  } else if (error instanceof NotFoundError) {
    console.log('Resource not found');
  } else if (error instanceof LumaDBError) {
    console.log('Database error:', error.code, error.message);
  }
}
```

**Python:**
```python
from lumadb.exceptions import LumaDBError, ValidationError, NotFoundError

try:
    users.insert({'name': 'Alice'})  # missing email
except ValidationError as e:
    print(f"Validation failed: {e.details}")
except NotFoundError as e:
    print("Resource not found")
except LumaDBError as e:
    print(f"Database error: {e.code} - {e.message}")
```

### 6.4 Retry Strategies

```typescript
const client = createClient({
  url: 'http://localhost:8080',
  retry: {
    maxRetries: 3,
    initialDelay: 100,
    maxDelay: 5000,
    backoffMultiplier: 2,
    retryableErrors: ['UNAVAILABLE', 'TIMEOUT', 'INTERNAL_ERROR']
  }
});
```

---

*Document Version: 3.0.0*
*Last Updated: December 2024*
