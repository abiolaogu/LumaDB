---
marp: true
theme: default
paginate: true
backgroundColor: #f8fafc
color: #1e293b
style: |
  section {
    font-family: 'Inter', 'Segoe UI', sans-serif;
  }
  h1 { color: #0f172a; }
  h2 { color: #3b82f6; }
  code { background: #e2e8f0; padding: 2px 6px; border-radius: 4px; }
  pre { background: #1e293b; color: #e2e8f0; }
---

# LumaDB Training
## Developer & Administrator Course

### Getting Started with LumaDB

---

# Course Outline

## What You'll Learn

| Module | Topic | Duration |
|--------|-------|----------|
| 1 | Introduction & Setup | 30 min |
| 2 | Basic Operations | 45 min |
| 3 | Query Languages | 60 min |
| 4 | Indexing & Performance | 45 min |
| 5 | AI & Vector Search | 45 min |
| 6 | Administration | 45 min |
| 7 | Hands-on Lab | 60 min |

**Total Duration: ~5.5 hours**

---

# Module 1: Introduction

## What is LumaDB?

LumaDB is a **unified database platform** that:

- Speaks **11 different database protocols**
- Provides **3 native query languages**
- Includes **built-in AI capabilities**
- Delivers **extreme performance**

### One database to replace many!

---

# Architecture Overview

## Multi-Language Design

```
┌─────────────────────────────────────────────────────────┐
│                      LumaDB                              │
├─────────────────────────────────────────────────────────┤
│  TypeScript SDK    Python AI     Go Cluster    Rust Core│
│  ─────────────     ─────────     ──────────    ───────  │
│  • Your code       • Vectors     • Distributed • Storage│
│  • Type-safe       • Embeddings  • Consensus   • Speed  │
│  • CLI/REPL        • PromptQL    • Routing     • SIMD   │
└─────────────────────────────────────────────────────────┘
```

---

# Installation

## Quick Start with Docker

```bash
# Pull and run LumaDB
docker run -d \
  --name lumadb \
  -p 8080:8080 \
  -p 5432:5432 \
  -v lumadb-data:/var/lib/lumadb \
  lumadb/lumadb:latest

# Verify it's running
curl http://localhost:8080/health
```

**That's it!** LumaDB is now running.

---

# Installation Options

## Choose Your Method

### Docker (Recommended for dev)
```bash
docker-compose up -d
```

### Kubernetes (Production)
```bash
kubectl apply -f k8s/
```

### npm (SDK only)
```bash
npm install tdb-plus
```

---

# Module 2: Basic Operations

## Connecting to LumaDB

### Option 1: TypeScript SDK
```typescript
import { Database } from 'tdb-plus';

const db = Database.create('my_app');
await db.open();
```

### Option 2: PostgreSQL Protocol
```bash
psql -h localhost -p 5432 -U admin -d lumadb
```

### Option 3: HTTP REST API
```bash
curl http://localhost:8080/api/v1/collections
```

---

# Creating Collections

## Collections = Tables

```typescript
// Get or create a collection
const users = db.collection('users');

// Collection with schema validation
const products = db.collection('products', {
  schema: {
    name: { type: 'string', required: true },
    price: { type: 'number', min: 0 },
    category: { type: 'string' }
  }
});
```

---

# Insert Operations

## Adding Data

```typescript
// Insert single document
const user = await users.insert({
  name: 'Alice Johnson',
  email: 'alice@example.com',
  age: 28
});

console.log('Created:', user._id);
// Output: Created: abc123-def456-...

// Insert multiple documents
const newUsers = await users.insertMany([
  { name: 'Bob', email: 'bob@example.com', age: 32 },
  { name: 'Carol', email: 'carol@example.com', age: 25 }
]);

console.log(`Created ${newUsers.length} users`);
```

---

# Read Operations

## Finding Data

```typescript
// Find by ID
const user = await users.findById('abc123');

// Find with conditions
const results = await users.find({
  where: {
    age: { $gte: 21 },
    status: 'active'
  },
  orderBy: { name: 'asc' },
  limit: 10
});

// Find one
const admin = await users.findOne({ role: 'admin' });

// Count
const count = await users.count({ status: 'active' });
```

---

# Update Operations

## Modifying Data

```typescript
// Update by ID
await users.updateById('abc123', {
  status: 'verified',
  verifiedAt: new Date()
});

// Update multiple documents
const result = await users.updateMany(
  { status: 'pending' },          // filter
  { status: 'active' }            // updates
);

console.log(`Updated ${result.modifiedCount} documents`);
```

---

# Delete Operations

## Removing Data

```typescript
// Delete by ID
await users.deleteById('abc123');

// Delete multiple
const result = await users.deleteMany({
  status: 'inactive',
  lastLogin: { $lt: '2023-01-01' }
});

console.log(`Deleted ${result.deletedCount} documents`);
```

---

# 🎯 Exercise 1: Basic CRUD

## Try It Yourself!

1. Create a `products` collection
2. Insert 5 products with name, price, category
3. Find all products under $50
4. Update one product's price
5. Delete products in category "discontinued"

```typescript
// Your code here...
const products = db.collection('products');

// Insert
await products.insert({ name: 'Widget', price: 29.99, category: 'tools' });

// Find, Update, Delete...
```

---

# Module 3: Query Languages

## Three Ways to Query

| Language | Style | Best For |
|----------|-------|----------|
| **LQL** | SQL-like | SQL developers |
| **NQL** | Natural language | Beginners |
| **JQL** | JSON-based | MongoDB developers |

**Use whichever feels most comfortable!**

---

# LQL: SQL-Like Queries

## Familiar SQL Syntax

```typescript
// SELECT queries
const users = await db.lql(`
  SELECT name, email, age
  FROM users
  WHERE status = 'active' AND age > 21
  ORDER BY name ASC
  LIMIT 10
`);

// Aggregations
const stats = await db.lql(`
  SELECT category, COUNT(*) as count, AVG(price) as avg_price
  FROM products
  GROUP BY category
  HAVING COUNT(*) > 5
`);
```

---

# LQL: Joins & Subqueries

## Advanced SQL Features

```typescript
// JOIN example
const ordersWithUsers = await db.lql(`
  SELECT o.id, o.total, u.name, u.email
  FROM orders o
  INNER JOIN users u ON o.user_id = u.id
  WHERE o.status = 'completed'
`);

// Subquery example
const highSpenders = await db.lql(`
  SELECT * FROM users
  WHERE id IN (
    SELECT user_id FROM orders
    GROUP BY user_id
    HAVING SUM(total) > 1000
  )
`);
```

---

# NQL: Natural Language Queries

## Write Queries in English!

```typescript
// Simple finds
const users = await db.nql('find all users');
const active = await db.nql('get users where status equals active');

// Sorting and limiting
const recent = await db.nql('show first 10 orders sorted by date descending');

// Aggregations
const count = await db.nql('count all products where price is greater than 50');

// Inserts
await db.nql('add to users name "Alice", email "alice@example.com"');

// Updates
await db.nql('update users set verified to true where email_confirmed is true');
```

---

# JQL: JSON Query Language

## MongoDB-Style Queries

```typescript
// Find with filters
const users = await db.jql({
  find: 'users',
  filter: {
    age: { $gte: 21, $lte: 65 },
    status: { $in: ['active', 'verified'] }
  },
  sort: { createdAt: -1 },
  limit: 10
});

// Aggregation pipeline
const analytics = await db.jql({
  aggregate: 'orders',
  pipeline: [
    { $match: { status: 'completed' } },
    { $group: { _id: '$category', total: { $sum: '$amount' } } },
    { $sort: { total: -1 } }
  ]
});
```

---

# 🎯 Exercise 2: Query Languages

## Try All Three!

Write the same query in LQL, NQL, and JQL:
**"Find all products in the 'electronics' category with price under $100, sorted by price"**

```typescript
// LQL version
await db.lql(`SELECT * FROM products WHERE category = 'electronics' AND price < 100 ORDER BY price`);

// NQL version
await db.nql('find products where category equals electronics and price is less than 100 sorted by price');

// JQL version
await db.jql({ find: 'products', filter: { category: 'electronics', price: { $lt: 100 } }, sort: { price: 1 } });
```

---

# Module 4: Indexing

## Why Indexes Matter

```
Without index:  Full collection scan  O(n)    SLOW!
With index:     Direct lookup         O(log n) FAST!
```

### Index Types in LumaDB

| Type | Use Case | Complexity |
|------|----------|------------|
| **B-Tree** | Range queries, sorting | O(log n) |
| **Hash** | Exact match lookups | O(1) |
| **Full-Text** | Text search | O(log n) |
| **Vector** | Similarity search | O(log n) |

---

# Creating Indexes

## Index Examples

```typescript
// B-Tree index for range queries
await users.createIndex({
  name: 'idx_users_age',
  fields: ['age'],
  type: 'btree'
});

// Unique index
await users.createIndex({
  name: 'idx_users_email',
  fields: ['email'],
  type: 'hash',
  unique: true
});

// Compound index
await orders.createIndex({
  name: 'idx_orders_user_date',
  fields: ['user_id', 'created_at'],
  type: 'btree'
});
```

---

# Full-Text Search

## Search Within Text Content

```typescript
// Create full-text index
await articles.createIndex({
  name: 'idx_articles_content',
  fields: ['title', 'body'],
  type: 'fulltext',
  options: {
    language: 'english',
    weights: { title: 2, body: 1 }
  }
});

// Search
const results = await db.lql(`
  SELECT * FROM articles
  WHERE MATCH(title, body) AGAINST ('database performance')
  ORDER BY _score DESC
  LIMIT 10
`);
```

---

# Query Optimization

## Explain Your Queries

```typescript
// See execution plan
const plan = await db.lql(`
  EXPLAIN SELECT * FROM orders
  WHERE user_id = 'user-123'
  AND created_at > '2024-01-01'
`);

console.log(plan);
// Output shows:
// - Index used (or full scan)
// - Estimated rows
// - Execution time
```

### Tips
- Add indexes for WHERE clause fields
- Use compound indexes for multi-field filters
- Avoid `SELECT *` - specify needed fields

---

# Module 5: AI & Vector Search

## Built-In AI Capabilities

LumaDB includes:
- **Vector Search** - Find similar items
- **Embeddings** - Convert text to vectors
- **PromptQL** - Natural language to SQL
- **LLM Integration** - OpenAI, Anthropic, etc.

---

# Setting Up AI Features

## Configure AI Provider

```typescript
const db = Database.create('ai_app', {
  ai: {
    provider: 'openai',
    apiKey: process.env.OPENAI_API_KEY,
    model: 'text-embedding-ada-002'
  }
});

await db.open();
```

---

# Vector Search: Indexing

## Add Documents for Semantic Search

```typescript
// Index a document with text
await db.ai.index('products', {
  id: 'prod-001',
  text: 'Wireless noise-canceling headphones with 30-hour battery',
  metadata: {
    category: 'electronics',
    price: 299.99,
    brand: 'AudioTech'
  }
});

// Index many documents
await db.ai.indexMany('products', [
  { id: 'prod-002', text: 'Gaming keyboard with RGB', metadata: {...} },
  { id: 'prod-003', text: 'Ergonomic office chair', metadata: {...} }
]);
```

---

# Vector Search: Querying

## Find Similar Items

```typescript
// Basic semantic search
const results = await db.ai.search('products', {
  query: 'comfortable headphones for long flights',
  topK: 10
});

// With filters
const filtered = await db.ai.search('products', {
  query: 'wireless audio device',
  topK: 5,
  filter: {
    category: 'electronics',
    price: { $lte: 200 }
  }
});

results.forEach(r => {
  console.log(`${r.id}: ${r.score.toFixed(3)} - ${r.metadata.name}`);
});
```

---

# PromptQL: Natural Language Queries

## Ask Questions in Plain English

```typescript
import { PromptQLEngine } from 'tdb-plus';

const promptql = new PromptQLEngine({
  db: db,
  llm: {
    provider: 'openai',
    model: 'gpt-4'
  }
});

// Natural language query
const result = await promptql.query(
  "Show me all customers who spent more than $500 last month"
);

console.log('Generated SQL:', result.query);
console.log('Results:', result.data);
```

---

# 🎯 Exercise 3: Vector Search

## Build a Product Recommender

1. Index 10 products with descriptions
2. Search for "laptop for software development"
3. Filter results by price < $1500

```typescript
// Index products
await db.ai.index('products', {
  id: 'laptop-1',
  text: 'MacBook Pro 14" M3 chip, 16GB RAM, for developers',
  metadata: { price: 1999, category: 'laptops' }
});

// Search
const recommendations = await db.ai.search('products', {
  query: 'laptop for software development',
  topK: 5,
  filter: { price: { $lt: 1500 } }
});
```

---

# Module 6: Administration

## Key Admin Tasks

- Monitoring & metrics
- Backup & recovery
- User management
- Configuration tuning

---

# Monitoring

## Health & Metrics

```bash
# Health check
curl http://localhost:8080/health

# Prometheus metrics
curl http://localhost:8080/metrics

# Statistics
curl http://localhost:8080/api/v1/stats
```

### Key Metrics to Watch
- Query latency (p50, p95, p99)
- Throughput (queries/sec)
- Cache hit rate
- Storage usage
- Connection count

---

# Backup & Restore

## Protect Your Data

```bash
# Full backup
lumadb backup \
  --destination /backups/$(date +%Y%m%d) \
  --type full

# Restore
lumadb restore \
  --source /backups/20240115 \
  --target-db production

# Point-in-time recovery
lumadb restore \
  --source /backups/20240115 \
  --point-in-time "2024-01-15T14:30:00Z"
```

---

# User Management

## Security Configuration

```bash
# Create user
lumadb user create --username alice --role developer

# List users
lumadb user list

# Update role
lumadb user update alice --role admin

# Disable user
lumadb user disable alice
```

### Roles
- **admin** - Full access
- **developer** - Read/write, no admin
- **analyst** - Read-only
- **custom** - Define your own

---

# Configuration Tuning

## Key Settings

```toml
# /etc/lumadb/config.toml

[memory]
memtable_size = 67108864      # 64 MB
block_cache_size = 536870912  # 512 MB

[wal]
sync_mode = "group_commit"
batch_size = 1000

[compaction]
style = "leveled"
max_background_jobs = 4
```

---

# Module 7: Hands-On Lab

## Build a Complete Application

### Scenario: E-Commerce Product Search

1. Create products collection
2. Add indexes for category and price
3. Index products for vector search
4. Build search API with filters
5. Add product recommendations

**Time: 60 minutes**

---

# Lab Setup

## Starting Point

```typescript
import { Database } from 'tdb-plus';

// Initialize database
const db = Database.create('ecommerce', {
  storage: 'file',
  path: './data',
  ai: {
    provider: 'openai',
    apiKey: process.env.OPENAI_API_KEY
  }
});

await db.open();

// Your code starts here...
```

---

# Lab Step 1: Create Collections

```typescript
// Products collection
const products = db.collection('products', {
  schema: {
    name: { type: 'string', required: true },
    description: { type: 'string', required: true },
    price: { type: 'number', required: true, min: 0 },
    category: { type: 'string', required: true },
    inStock: { type: 'boolean', default: true }
  }
});

// Orders collection
const orders = db.collection('orders');
```

---

# Lab Step 2: Add Indexes

```typescript
// Index for category filtering
await products.createIndex({
  name: 'idx_category',
  fields: ['category'],
  type: 'btree'
});

// Index for price range queries
await products.createIndex({
  name: 'idx_price',
  fields: ['price'],
  type: 'btree'
});

// Full-text search on name and description
await products.createIndex({
  name: 'idx_search',
  fields: ['name', 'description'],
  type: 'fulltext'
});
```

---

# Lab Step 3: Insert Sample Data

```typescript
const sampleProducts = [
  { name: 'MacBook Pro 14"', description: 'Apple laptop with M3 chip', price: 1999, category: 'laptops' },
  { name: 'iPhone 15 Pro', description: 'Latest Apple smartphone', price: 999, category: 'phones' },
  { name: 'AirPods Pro', description: 'Wireless noise-canceling earbuds', price: 249, category: 'audio' },
  { name: 'iPad Air', description: 'Versatile tablet for work and play', price: 599, category: 'tablets' },
  { name: 'Sony WH-1000XM5', description: 'Premium wireless headphones', price: 399, category: 'audio' }
];

await products.insertMany(sampleProducts);
```

---

# Lab Step 4: Vector Search Setup

```typescript
// Index products for semantic search
const allProducts = await products.find({});

for (const product of allProducts) {
  await db.ai.index('products_vectors', {
    id: product._id,
    text: `${product.name} ${product.description}`,
    metadata: {
      name: product.name,
      price: product.price,
      category: product.category
    }
  });
}

console.log('Vector index created!');
```

---

# Lab Step 5: Build Search Function

```typescript
async function searchProducts(query, filters = {}) {
  // Semantic search
  const results = await db.ai.search('products_vectors', {
    query: query,
    topK: 10,
    filter: filters
  });

  return results.map(r => ({
    id: r.id,
    name: r.metadata.name,
    price: r.metadata.price,
    category: r.metadata.category,
    relevance: r.score
  }));
}

// Test it!
const results = await searchProducts('wireless headphones', { price: { $lt: 300 } });
console.log(results);
```

---

# Lab Complete!

## What You Built

✅ Product collection with schema validation
✅ Optimized indexes for filtering
✅ Full-text search capability
✅ AI-powered semantic search
✅ Filtered recommendations

### Next Steps
- Add user authentication
- Build REST API endpoints
- Add shopping cart functionality
- Implement order processing

---

# Summary

## What You Learned

| Module | Key Takeaways |
|--------|---------------|
| Setup | Docker, SDK installation |
| CRUD | Insert, find, update, delete |
| Query Languages | LQL, NQL, JQL |
| Indexing | B-Tree, Hash, Full-text |
| AI Features | Vector search, PromptQL |
| Administration | Monitoring, backup, security |

---

# Resources

## Continue Learning

### Documentation
📖 https://docs.lumadb.io

### API Reference
📘 https://api.lumadb.io

### GitHub
💻 https://github.com/lumadb

### Community
💬 https://discord.gg/lumadb

### Support
📧 support@lumadb.io

---

# Thank You!

## Questions?

### Certification
Complete the assessment at:
https://learn.lumadb.io/certification

### Feedback
Help us improve this training:
https://feedback.lumadb.io/training

---

# Appendix: Cheat Sheet

## Quick Reference

```typescript
// Connect
const db = Database.create('name');
await db.open();

// CRUD
await collection.insert({...});
await collection.find({ where: {...} });
await collection.updateById(id, {...});
await collection.deleteById(id);

// Query languages
await db.lql('SELECT * FROM ...');
await db.nql('find all users...');
await db.jql({ find: 'collection', filter: {...} });

// AI
await db.ai.index('name', { id, text, metadata });
await db.ai.search('name', { query, topK, filter });
```
