# TDB+ (Turbo Database Plus)

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.0-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <img src="https://img.shields.io/badge/node-%3E%3D18.0.0-brightgreen.svg" alt="Node">
</p>

**TDB+** is a modern, user-friendly database designed to be accessible to everyone - from beginners to enterprise developers. It features **three intuitive query languages**, making database operations feel natural regardless of your background.

## Why TDB+?

- **Easy to Learn**: Choose from SQL-like, Natural Language, or JSON queries
- **Enterprise-Grade**: ACID transactions, multiple isolation levels, advanced indexing
- **Developer-Friendly**: TypeScript-first, comprehensive SDK, excellent error messages
- **Flexible Storage**: In-memory for development, file-based for production
- **Modern Architecture**: Event-driven, real-time subscriptions, query optimization

## Quick Start

### Installation

```bash
npm install tdb-plus
```

### Basic Usage

```typescript
import { Database } from 'tdb-plus';

// Create a database
const db = Database.create('my_app');
await db.open();

// Choose your preferred query language!

// TQL (SQL-like) - for SQL users
await db.tql(`INSERT INTO users (name, email, age) VALUES ('Alice', 'alice@example.com', 28)`);
const users = await db.tql(`SELECT * FROM users WHERE age > 21`);

// NQL (Natural Language) - for beginners
await db.nql(`add to users name "Bob", email "bob@example.com", age 32`);
const activeUsers = await db.nql(`find all users where age is greater than 25`);

// JQL (JSON) - for MongoDB/NoSQL developers
await db.jql(`{ "insert": "users", "documents": [{ "name": "Charlie", "age": 25 }] }`);
const results = await db.jql(`{ "find": "users", "filter": { "age": { "$gt": 20 } } }`);

await db.close();
```

## Three Query Languages

### TQL (TDB Query Language) - SQL-Like

For developers familiar with SQL databases. Full SQL-like syntax with familiar keywords.

```sql
-- Select with conditions
SELECT * FROM users WHERE age > 21 AND status = 'active' ORDER BY name ASC LIMIT 10

-- Insert data
INSERT INTO products (name, price, category) VALUES ('Laptop', 999.99, 'Electronics')

-- Update with conditions
UPDATE users SET status = 'verified' WHERE email_verified = true

-- Delete records
DELETE FROM sessions WHERE expired_at < NOW()

-- Aggregations
SELECT category, COUNT(*), AVG(price) FROM products GROUP BY category HAVING COUNT(*) > 5

-- Create indexes
CREATE INDEX idx_users_email ON users (email) UNIQUE
```

### NQL (Natural Query Language) - Human-Readable

For beginners or anyone who prefers natural language. Reads like English!

```
-- Finding data
find all users
get users where age is greater than 21
show first 10 products sorted by price descending
find users where name contains "John"
get orders where status equals "pending" and total is greater than 100

-- Counting
count all users
how many orders where status equals "completed"

-- Inserting
add to users name "Jane", email "jane@example.com", age 28

-- Updating
update users set status to "active" where verified is true
modify products set price to 29.99 where name equals "Widget"

-- Deleting
remove users where inactive is true
delete all sessions where expired is true
```

### JQL (JSON Query Language) - MongoDB-Style

For developers from NoSQL backgrounds. Familiar JSON syntax with MongoDB-like operators.

```json
// Find with filter and options
{
  "find": "users",
  "filter": { "age": { "$gt": 21 }, "status": "active" },
  "sort": { "createdAt": -1 },
  "limit": 10,
  "projection": { "name": 1, "email": 1 }
}

// Insert documents
{
  "insert": "users",
  "documents": [
    { "name": "Alice", "email": "alice@example.com", "age": 28 },
    { "name": "Bob", "email": "bob@example.com", "age": 32 }
  ]
}

// Update with filter
{
  "update": "users",
  "filter": { "email": "alice@example.com" },
  "set": { "status": "premium", "updatedAt": "2024-01-15" }
}

// Aggregation pipeline
{
  "aggregate": "orders",
  "pipeline": [
    { "$match": { "status": "completed" } },
    { "$group": { "_id": "$product", "total": { "$sum": "$amount" } } },
    { "$sort": { "total": -1 } }
  ]
}
```

## Enterprise Features

### ACID Transactions

Full transaction support with multiple isolation levels:

```typescript
// Automatic transaction (commit on success, rollback on error)
await db.transaction(async (tx) => {
  const users = tx.collection('users');
  const accounts = tx.collection('accounts');

  await users.insert({ name: 'Alice', accountId: 'acc_123' });
  await accounts.updateById('acc_123', { balance: 1000 });
});

// Manual transaction control
const tx = await db.beginTransaction({
  isolationLevel: 'SERIALIZABLE',
  timeout: 30000,
});

try {
  await tx.collection('orders').insert({ product: 'Widget', quantity: 5 });
  await tx.collection('inventory').updateById('widget', { stock: 95 });
  await tx.commit();
} catch (error) {
  await tx.rollback();
  throw error;
}
```

### Advanced Indexing

Multiple index types for optimal query performance:

```typescript
const users = db.collection('users');

// B-Tree index (range queries, sorting)
await users.createIndex('idx_age', ['age'], 'btree');

// Hash index (exact lookups)
await users.createIndex('idx_email', ['email'], 'hash', { unique: true });

// Full-text index (text search)
await users.createIndex('idx_bio', ['bio', 'interests'], 'fulltext');

// Composite index
await users.createIndex('idx_name_age', ['lastName', 'firstName', 'age'], 'btree');
```

### Query Optimization

Automatic query planning and optimization:

```typescript
// Explain a query
const plan = await db.explain('SELECT * FROM users WHERE age > 21 ORDER BY name');
console.log(plan.queryPlan);
// {
//   steps: [
//     { operation: 'INDEX_SCAN', description: 'Scan index: idx_age', estimatedRows: 100 },
//     { operation: 'FILTER', description: 'Apply 1 condition(s)', estimatedRows: 50 },
//     { operation: 'SORT', description: 'Sort by name ASC', estimatedRows: 50 }
//   ],
//   estimatedCost: 260,
//   indexesUsed: ['idx_age']
// }
```

### Real-Time Events

Subscribe to database events:

```typescript
// Subscribe to document changes
db.on('document:created', (event) => {
  console.log('New document:', event.data);
});

db.on('document:updated', (event) => {
  console.log('Updated:', event.data.documentId);
});

db.on('query:executed', (event) => {
  console.log(`Query took ${event.data.executionTime}ms`);
});
```

## CLI & REPL

Interactive command-line interface with syntax highlighting and auto-completion:

```bash
# Start the REPL
npx tdb

# Or with a database file
npx tdb --db ./mydata
```

```
╔══════════════════════════════════════════════════════════════════╗
║   TDB+  PLUS  -  The Modern, User-Friendly Database              ║
╚══════════════════════════════════════════════════════════════════╝

TQL > SELECT * FROM users WHERE age > 21
┌──────────┬─────────────────────┬─────┐
│ name     │ email               │ age │
├──────────┼─────────────────────┼─────┤
│ Alice    │ alice@example.com   │ 28  │
│ Bob      │ bob@example.com     │ 32  │
└──────────┴─────────────────────┴─────┘
2 row(s) returned in 3ms

TQL > .nql
Switched to NQL (Natural Language)

NQL > find users where name contains "Alice"
...
```

### CLI Commands

| Command | Description |
|---------|-------------|
| `.help` | Show help |
| `.tql`, `.nql`, `.jql` | Switch query language |
| `.collections` | List all collections |
| `.stats` | Show database statistics |
| `.examples` | Show query examples |
| `.tutorial` | Interactive tutorial |
| `.clear` | Clear screen |
| `.exit` | Exit TDB+ |

## HTTP Server

Run TDB+ as a REST API server:

```bash
# Start the server
npx tdb-server

# With options
TDB_PORT=3000 TDB_PATH=./data npx tdb-server
```

### API Endpoints

```bash
# Execute a query
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM users", "language": "tql"}'

# List collections
curl http://localhost:3000/collections

# Get statistics
curl http://localhost:3000/stats

# Health check
curl http://localhost:3000/health
```

## Client SDK

### JavaScript/TypeScript

```typescript
import { createClient } from 'tdb-plus/client';

const client = createClient('http://localhost:3000');

// Query with TQL
const users = await client.tql('SELECT * FROM users');

// Query with NQL
const orders = await client.nql('find all orders where status equals "pending"');

// Query with JQL (accepts objects directly)
const products = await client.jql({
  find: 'products',
  filter: { price: { $lt: 100 } },
  sort: { price: 1 }
});

// Fluent query builder
const results = await client
  .collection('users')
  .where('age', '>', 21)
  .where('status', 'active')
  .orderBy('name', 'ASC')
  .limit(10)
  .get();

// Insert with builder
await client.collection('users').insert({
  name: 'Alice',
  email: 'alice@example.com',
  age: 28
});

// Update with conditions
await client
  .collection('users')
  .where('email', 'alice@example.com')
  .update({ status: 'premium' });
```

## Storage Options

### In-Memory (Default)

Fast storage for development and testing:

```typescript
const db = Database.create('my_app');
```

### File-Based (Persistent)

Durable storage with Write-Ahead Logging:

```typescript
const db = Database.createPersistent('my_app', './data');
```

### Configuration Options

```typescript
const db = new Database({
  name: 'my_app',
  storage: {
    type: 'file',        // 'memory' | 'file' | 'hybrid'
    path: './data',
    cacheSize: 1000,
    compression: true,
  },
  defaultQueryLanguage: 'tql',
  queryTimeout: 30000,
  cacheEnabled: true,
  logging: {
    level: 'info',
    destination: 'console',
  },
});
```

## API Reference

### Database

| Method | Description |
|--------|-------------|
| `open()` | Open database connection |
| `close()` | Close database connection |
| `collection(name)` | Get or create a collection |
| `dropCollection(name)` | Delete a collection |
| `query(q)` | Execute query with default language |
| `tql(q)` | Execute TQL query |
| `nql(q)` | Execute NQL query |
| `jql(q)` | Execute JQL query |
| `beginTransaction(opts)` | Start a transaction |
| `transaction(fn)` | Execute function in transaction |
| `on(event, handler)` | Subscribe to events |
| `getStats()` | Get database statistics |

### Collection

| Method | Description |
|--------|-------------|
| `insert(data)` | Insert a document |
| `insertMany(data)` | Insert multiple documents |
| `findById(id)` | Find document by ID |
| `find(options)` | Find documents with conditions |
| `findOne(options)` | Find first matching document |
| `updateById(id, data)` | Update document by ID |
| `updateMany(conditions, data)` | Update matching documents |
| `deleteById(id)` | Delete document by ID |
| `deleteMany(conditions)` | Delete matching documents |
| `count(conditions)` | Count matching documents |
| `createIndex(name, fields, type)` | Create an index |
| `dropIndex(name)` | Drop an index |

### Document

| Property/Method | Description |
|-----------------|-------------|
| `id` | Document ID |
| `revision` | Revision number |
| `createdAt` | Creation timestamp |
| `updatedAt` | Last update timestamp |
| `data` | Document data |
| `get(field)` | Get field value |
| `set(field, value)` | Set field value |
| `save()` | Save changes |
| `delete()` | Delete document |
| `toObject()` | Convert to plain object |

## Comparison with Other Databases

| Feature | TDB+ | SQLite | MongoDB | PostgreSQL |
|---------|------|--------|---------|------------|
| Learning Curve | Easy | Medium | Medium | Hard |
| Query Languages | 3 | 1 | 1 | 1 |
| Natural Language Queries | Yes | No | No | No |
| ACID Transactions | Yes | Yes | Yes* | Yes |
| Full-Text Search | Built-in | Extension | Yes | Extension |
| TypeScript Native | Yes | No | Yes | No |
| Embedded Mode | Yes | Yes | No | No |
| Server Mode | Yes | No | Yes | Yes |
| Real-time Events | Yes | No | Yes | Limited |

## Performance

TDB+ is optimized for developer productivity while maintaining good performance:

- **Indexing**: B-Tree, Hash, and Full-Text indexes for fast queries
- **Query Caching**: Parsed queries are cached for repeated execution
- **WAL**: Write-Ahead Logging for crash recovery
- **Lazy Loading**: Collections are loaded on demand

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  Made with love for developers who deserve better databases.
</p>
