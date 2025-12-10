/**
 * TDB+ JQL Parser (JSON Query Language)
 *
 * MongoDB-style JSON query language for developers who prefer
 * structured, programmatic query building.
 *
 * Examples:
 *   { "find": "users", "filter": { "age": { "$gt": 21 } } }
 *   { "find": "users", "filter": { "name": { "$regex": "^John" } }, "sort": { "age": -1 }, "limit": 10 }
 *   { "insert": "users", "documents": [{ "name": "John", "age": 25 }] }
 *   { "update": "users", "filter": { "id": "123" }, "set": { "status": "active" } }
 *   { "delete": "users", "filter": { "inactive": true } }
 *   { "aggregate": "orders", "pipeline": [{ "$match": { "status": "completed" } }, { "$group": { "_id": "$product", "total": { "$sum": "$amount" } } }] }
 *   { "count": "users", "filter": { "status": "active" } }
 */

import {
  ParsedQuery,
  QueryCondition,
  OrderByClause,
  AggregationClause,
  ComparisonOperator,
  QuerySyntaxError,
} from '../../types';

interface JQLQuery {
  // Query type indicators
  find?: string;
  insert?: string;
  update?: string;
  delete?: string;
  count?: string;
  aggregate?: string;
  createCollection?: string;
  dropCollection?: string;
  createIndex?: string;
  dropIndex?: string;

  // Query options
  filter?: Record<string, any>;
  projection?: Record<string, number | boolean>;
  sort?: Record<string, number>;
  limit?: number;
  skip?: number;
  offset?: number;

  // Insert options
  documents?: Record<string, any>[];
  document?: Record<string, any>;

  // Update options
  set?: Record<string, any>;
  unset?: string[];
  upsert?: boolean;

  // Aggregation pipeline
  pipeline?: Record<string, any>[];

  // Index options
  collection?: string;
  fields?: string[];
  unique?: boolean;
  name?: string;
}

export class JQLParser {
  /**
   * Parse a JQL query string
   */
  parse(queryString: string): ParsedQuery {
    let query: JQLQuery;

    try {
      query = JSON.parse(queryString);
    } catch (error) {
      throw new QuerySyntaxError(
        `Invalid JSON: ${error instanceof Error ? error.message : 'Parse error'}`,
        0,
        'Make sure your query is valid JSON'
      );
    }

    // Determine query type
    if (query.find) {
      return this.parseFind(query);
    }
    if (query.insert) {
      return this.parseInsert(query);
    }
    if (query.update) {
      return this.parseUpdate(query);
    }
    if (query.delete) {
      return this.parseDelete(query);
    }
    if (query.count) {
      return this.parseCount(query);
    }
    if (query.aggregate) {
      return this.parseAggregate(query);
    }
    if (query.createCollection) {
      return this.parseCreateCollection(query);
    }
    if (query.dropCollection) {
      return this.parseDropCollection(query);
    }
    if (query.createIndex) {
      return this.parseCreateIndex(query);
    }
    if (query.dropIndex) {
      return this.parseDropIndex(query);
    }

    throw new QuerySyntaxError(
      'Unknown query type. Use one of: find, insert, update, delete, count, aggregate',
      0,
      'Example: { "find": "users", "filter": { "age": { "$gt": 21 } } }'
    );
  }

  // ============================================================================
  // Query Parsers
  // ============================================================================

  private parseFind(query: JQLQuery): ParsedQuery {
    const result: ParsedQuery = {
      type: 'SELECT',
      collection: query.find!,
    };

    if (query.filter) {
      result.conditions = this.parseFilter(query.filter);
    }

    if (query.projection) {
      result.fields = this.parseProjection(query.projection);
    }

    if (query.sort) {
      result.orderBy = this.parseSort(query.sort);
    }

    if (query.limit !== undefined) {
      result.limit = query.limit;
    }

    if (query.skip !== undefined || query.offset !== undefined) {
      result.offset = query.skip ?? query.offset;
    }

    return result;
  }

  private parseInsert(query: JQLQuery): ParsedQuery {
    const documents = query.documents || (query.document ? [query.document] : []);

    if (documents.length === 0) {
      throw new QuerySyntaxError(
        'Insert requires "documents" or "document" field',
        0,
        'Example: { "insert": "users", "documents": [{ "name": "John" }] }'
      );
    }

    return {
      type: 'INSERT',
      collection: query.insert!,
      data: documents.length === 1 ? documents[0] : documents,
    };
  }

  private parseUpdate(query: JQLQuery): ParsedQuery {
    if (!query.set && !query.unset) {
      throw new QuerySyntaxError(
        'Update requires "set" or "unset" field',
        0,
        'Example: { "update": "users", "filter": { "id": "123" }, "set": { "name": "Jane" } }'
      );
    }

    const result: ParsedQuery = {
      type: 'UPDATE',
      collection: query.update!,
      data: query.set || {},
    };

    if (query.filter) {
      result.conditions = this.parseFilter(query.filter);
    }

    return result;
  }

  private parseDelete(query: JQLQuery): ParsedQuery {
    const result: ParsedQuery = {
      type: 'DELETE',
      collection: query.delete!,
    };

    if (query.filter) {
      result.conditions = this.parseFilter(query.filter);
    }

    return result;
  }

  private parseCount(query: JQLQuery): ParsedQuery {
    const result: ParsedQuery = {
      type: 'COUNT',
      collection: query.count!,
    };

    if (query.filter) {
      result.conditions = this.parseFilter(query.filter);
    }

    return result;
  }

  private parseAggregate(query: JQLQuery): ParsedQuery {
    const result: ParsedQuery = {
      type: 'AGGREGATE',
      collection: query.aggregate!,
      aggregations: [],
    };

    if (!query.pipeline || !Array.isArray(query.pipeline)) {
      throw new QuerySyntaxError(
        'Aggregate requires "pipeline" array',
        0,
        'Example: { "aggregate": "orders", "pipeline": [{ "$match": {...} }, { "$group": {...} }] }'
      );
    }

    // Process pipeline stages
    for (const stage of query.pipeline) {
      if (stage.$match) {
        result.conditions = this.parseFilter(stage.$match);
      }

      if (stage.$group) {
        this.parseGroupStage(stage.$group, result);
      }

      if (stage.$sort) {
        result.orderBy = this.parseSort(stage.$sort);
      }

      if (stage.$limit) {
        result.limit = stage.$limit;
      }

      if (stage.$skip) {
        result.offset = stage.$skip;
      }

      if (stage.$project) {
        result.fields = this.parseProjection(stage.$project);
      }
    }

    return result;
  }

  private parseCreateCollection(query: JQLQuery): ParsedQuery {
    return {
      type: 'CREATE_COLLECTION',
      collection: query.createCollection!,
    };
  }

  private parseDropCollection(query: JQLQuery): ParsedQuery {
    return {
      type: 'DROP_COLLECTION',
      collection: query.dropCollection!,
    };
  }

  private parseCreateIndex(query: JQLQuery): ParsedQuery {
    if (!query.collection || !query.fields) {
      throw new QuerySyntaxError(
        'createIndex requires "collection" and "fields"',
        0,
        'Example: { "createIndex": "idx_name", "collection": "users", "fields": ["name"] }'
      );
    }

    return {
      type: 'CREATE_INDEX',
      collection: query.collection,
      data: {
        name: query.createIndex,
        fields: query.fields,
        unique: query.unique || false,
      },
    };
  }

  private parseDropIndex(query: JQLQuery): ParsedQuery {
    if (!query.collection) {
      throw new QuerySyntaxError(
        'dropIndex requires "collection"',
        0,
        'Example: { "dropIndex": "idx_name", "collection": "users" }'
      );
    }

    return {
      type: 'DROP_INDEX',
      collection: query.collection,
      data: { name: query.dropIndex },
    };
  }

  // ============================================================================
  // Filter Parsing
  // ============================================================================

  private parseFilter(filter: Record<string, any>): QueryCondition[] {
    const conditions: QueryCondition[] = [];

    for (const [key, value] of Object.entries(filter)) {
      // Handle logical operators
      if (key === '$and') {
        if (Array.isArray(value)) {
          for (const subFilter of value) {
            conditions.push(...this.parseFilter(subFilter));
          }
        }
        continue;
      }

      if (key === '$or') {
        if (Array.isArray(value)) {
          for (let i = 0; i < value.length; i++) {
            const subConditions = this.parseFilter(value[i]);
            for (const cond of subConditions) {
              if (i > 0) cond.logic = 'OR';
              conditions.push(cond);
            }
          }
        }
        continue;
      }

      // Handle field conditions
      if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        // MongoDB-style operators
        for (const [op, opValue] of Object.entries(value)) {
          const condition = this.parseOperator(key, op, opValue);
          if (condition) {
            conditions.push(condition);
          }
        }
      } else {
        // Simple equality
        conditions.push({
          field: key,
          operator: '=',
          value: value,
        });
      }
    }

    return conditions;
  }

  private parseOperator(field: string, operator: string, value: any): QueryCondition | null {
    const operatorMap: Record<string, ComparisonOperator> = {
      $eq: '=',
      $ne: '!=',
      $gt: '>',
      $gte: '>=',
      $lt: '<',
      $lte: '<=',
      $in: 'IN',
      $nin: 'NOT IN',
      $regex: 'MATCHES',
      $like: 'LIKE',
      $contains: 'CONTAINS',
      $startsWith: 'STARTS WITH',
      $endsWith: 'ENDS WITH',
      $exists: value ? 'IS NOT NULL' : 'IS NULL',
    };

    const mappedOperator = operatorMap[operator];
    if (mappedOperator) {
      return {
        field,
        operator: mappedOperator,
        value: operator === '$exists' ? null : value,
      };
    }

    // Handle $between
    if (operator === '$between' && Array.isArray(value) && value.length === 2) {
      return {
        field,
        operator: 'BETWEEN',
        value: value,
      };
    }

    // Handle $not
    if (operator === '$not' && typeof value === 'object') {
      const innerConditions = this.parseOperator(field, Object.keys(value)[0], Object.values(value)[0]);
      if (innerConditions) {
        // Negate the operator
        switch (innerConditions.operator) {
          case '=':
            innerConditions.operator = '!=';
            break;
          case '!=':
            innerConditions.operator = '=';
            break;
          case '>':
            innerConditions.operator = '<=';
            break;
          case '>=':
            innerConditions.operator = '<';
            break;
          case '<':
            innerConditions.operator = '>=';
            break;
          case '<=':
            innerConditions.operator = '>';
            break;
          case 'IN':
            innerConditions.operator = 'NOT IN';
            break;
          case 'LIKE':
            innerConditions.operator = 'NOT LIKE';
            break;
          case 'IS NULL':
            innerConditions.operator = 'IS NOT NULL';
            break;
          case 'IS NOT NULL':
            innerConditions.operator = 'IS NULL';
            break;
        }
        return innerConditions;
      }
    }

    console.warn(`Unknown operator: ${operator}`);
    return null;
  }

  // ============================================================================
  // Other Parsers
  // ============================================================================

  private parseProjection(projection: Record<string, number | boolean>): string[] {
    const fields: string[] = [];

    for (const [field, include] of Object.entries(projection)) {
      if (include === 1 || include === true) {
        fields.push(field);
      }
    }

    return fields.length > 0 ? fields : ['*'];
  }

  private parseSort(sort: Record<string, number>): OrderByClause[] {
    const clauses: OrderByClause[] = [];

    for (const [field, direction] of Object.entries(sort)) {
      clauses.push({
        field,
        direction: direction === -1 ? 'DESC' : 'ASC',
      });
    }

    return clauses;
  }

  private parseGroupStage(group: Record<string, any>, result: ParsedQuery): void {
    // Parse _id (group by field)
    if (group._id) {
      if (typeof group._id === 'string' && group._id.startsWith('$')) {
        result.groupBy = [group._id.slice(1)];
      } else if (typeof group._id === 'object') {
        result.groupBy = Object.values(group._id).map((v: any) =>
          typeof v === 'string' && v.startsWith('$') ? v.slice(1) : v
        );
      }
    }

    // Parse aggregation functions
    if (!result.aggregations) {
      result.aggregations = [];
    }

    for (const [alias, aggDef] of Object.entries(group)) {
      if (alias === '_id') continue;

      if (typeof aggDef === 'object') {
        const aggOp = Object.keys(aggDef)[0];
        const aggField = aggDef[aggOp];
        const field = typeof aggField === 'string' && aggField.startsWith('$')
          ? aggField.slice(1)
          : aggField;

        const functionMap: Record<string, AggregationClause['function']> = {
          $sum: 'SUM',
          $avg: 'AVG',
          $min: 'MIN',
          $max: 'MAX',
          $count: 'COUNT',
          $first: 'FIRST',
          $last: 'LAST',
          $push: 'ARRAY_AGG',
        };

        const func = functionMap[aggOp];
        if (func) {
          result.aggregations.push({
            function: func,
            field: field || '*',
            alias,
          });
        }
      }
    }
  }
}
