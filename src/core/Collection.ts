/**
 * TDB+ Collection - Document Collection Management
 *
 * A collection is analogous to a table in relational databases.
 * It stores documents and provides CRUD operations with indexing support.
 */

import { v4 as uuidv4 } from 'uuid';
import { Document } from './Document';
import { StorageEngine } from '../storage/StorageEngine';
import { IndexManager } from '../indexing/IndexManager';
import { Database } from './Database';
import {
  DocumentId,
  DocumentData,
  StoredDocument,
  QueryCondition,
  OrderByClause,
  IndexDefinition,
  IndexType,
  DocumentNotFoundError,
  ValidationError,
} from '../types';

export interface FindOptions {
  conditions?: QueryCondition[];
  orderBy?: OrderByClause[];
  limit?: number;
  offset?: number;
  fields?: string[];
}

export interface UpdateOptions {
  upsert?: boolean;
  returnNew?: boolean;
}

export class Collection {
  private name: string;
  private storage: StorageEngine;
  private indexManager: IndexManager;
  private database: Database;
  private documents: Map<DocumentId, StoredDocument>;
  private isLoaded: boolean;

  constructor(
    name: string,
    storage: StorageEngine,
    indexManager: IndexManager,
    database: Database
  ) {
    this.name = name;
    this.storage = storage;
    this.indexManager = indexManager;
    this.database = database;
    this.documents = new Map();
    this.isLoaded = false;
  }

  // ============================================================================
  // CRUD Operations
  // ============================================================================

  /**
   * Insert a single document
   */
  async insert(data: DocumentData): Promise<Document> {
    await this.ensureLoaded();

    const id = data._id || uuidv4();
    if (this.documents.has(id)) {
      throw new ValidationError(`Document with ID ${id} already exists`, '_id');
    }

    const now = new Date();
    const stored: StoredDocument = {
      _id: id,
      _rev: 1,
      _createdAt: now,
      _updatedAt: now,
      data: { ...data },
    };
    delete stored.data._id; // Don't store _id in data

    this.documents.set(id, stored);
    await this.storage.saveDocument(this.name, stored);

    // Update indexes
    this.indexManager.indexDocument(this.name, id, stored.data);

    this.database.emit('document:created', {
      collection: this.name,
      documentId: id,
      data: stored.data,
    });

    return new Document(stored, this);
  }

  /**
   * Insert multiple documents
   */
  async insertMany(dataArray: DocumentData[]): Promise<Document[]> {
    const results: Document[] = [];
    for (const data of dataArray) {
      results.push(await this.insert(data));
    }
    return results;
  }

  /**
   * Find a document by ID
   */
  async findById(id: DocumentId): Promise<Document | null> {
    await this.ensureLoaded();

    const stored = this.documents.get(id);
    if (!stored || stored._deleted) {
      return null;
    }

    return new Document(stored, this);
  }

  /**
   * Get a document by ID (throws if not found)
   */
  async getById(id: DocumentId): Promise<Document> {
    const doc = await this.findById(id);
    if (!doc) {
      throw new DocumentNotFoundError(this.name, id);
    }
    return doc;
  }

  /**
   * Find documents matching conditions
   */
  async find(options: FindOptions = {}): Promise<Document[]> {
    await this.ensureLoaded();

    let results: StoredDocument[] = [];

    // Check if we can use an index
    const indexResults = this.tryUseIndex(options.conditions || []);

    if (indexResults !== null) {
      // Use index results
      results = indexResults
        .map((id) => this.documents.get(id))
        .filter((doc): doc is StoredDocument => doc !== undefined && !doc._deleted);
    } else {
      // Full scan
      results = Array.from(this.documents.values()).filter((doc) => !doc._deleted);
    }

    // Apply conditions
    if (options.conditions && options.conditions.length > 0) {
      results = results.filter((doc) => this.matchesConditions(doc.data, options.conditions!));
    }

    // Apply ordering
    if (options.orderBy && options.orderBy.length > 0) {
      results = this.sortDocuments(results, options.orderBy);
    }

    // Apply offset and limit
    if (options.offset) {
      results = results.slice(options.offset);
    }
    if (options.limit) {
      results = results.slice(0, options.limit);
    }

    // Apply field projection
    if (options.fields && options.fields.length > 0) {
      results = results.map((doc) => ({
        ...doc,
        data: this.projectFields(doc.data, options.fields!),
      }));
    }

    return results.map((stored) => new Document(stored, this));
  }

  /**
   * Find one document matching conditions
   */
  async findOne(options: FindOptions = {}): Promise<Document | null> {
    const results = await this.find({ ...options, limit: 1 });
    return results[0] || null;
  }

  /**
   * Update a document by ID
   */
  async updateById(
    id: DocumentId,
    updates: DocumentData,
    options: UpdateOptions = {}
  ): Promise<Document | null> {
    await this.ensureLoaded();

    const stored = this.documents.get(id);

    if (!stored || stored._deleted) {
      if (options.upsert) {
        return this.insert({ _id: id, ...updates });
      }
      return null;
    }

    const oldData = { ...stored.data };
    const newData = { ...stored.data, ...updates };

    stored.data = newData;
    stored._rev += 1;
    stored._updatedAt = new Date();

    await this.storage.saveDocument(this.name, stored);

    // Update indexes
    this.indexManager.unindexDocument(this.name, id, oldData);
    this.indexManager.indexDocument(this.name, id, newData);

    this.database.emit('document:updated', {
      collection: this.name,
      documentId: id,
      oldData,
      newData,
    });

    return new Document(stored, this);
  }

  /**
   * Update multiple documents matching conditions
   */
  async updateMany(
    conditions: QueryCondition[],
    updates: DocumentData
  ): Promise<{ modified: number; documents: Document[] }> {
    await this.ensureLoaded();

    const toUpdate = Array.from(this.documents.values()).filter(
      (doc) => !doc._deleted && this.matchesConditions(doc.data, conditions)
    );

    const documents: Document[] = [];
    for (const stored of toUpdate) {
      const doc = await this.updateById(stored._id, updates);
      if (doc) {
        documents.push(doc);
      }
    }

    return { modified: documents.length, documents };
  }

  /**
   * Delete a document by ID
   */
  async deleteById(id: DocumentId): Promise<boolean> {
    await this.ensureLoaded();

    const stored = this.documents.get(id);
    if (!stored || stored._deleted) {
      return false;
    }

    // Soft delete
    stored._deleted = true;
    stored._updatedAt = new Date();

    await this.storage.deleteDocument(this.name, id);

    // Remove from indexes
    this.indexManager.unindexDocument(this.name, id, stored.data);

    this.database.emit('document:deleted', {
      collection: this.name,
      documentId: id,
      data: stored.data,
    });

    return true;
  }

  /**
   * Delete multiple documents matching conditions
   */
  async deleteMany(conditions: QueryCondition[]): Promise<number> {
    await this.ensureLoaded();

    const toDelete = Array.from(this.documents.values()).filter(
      (doc) => !doc._deleted && this.matchesConditions(doc.data, conditions)
    );

    let deleted = 0;
    for (const stored of toDelete) {
      if (await this.deleteById(stored._id)) {
        deleted++;
      }
    }

    return deleted;
  }

  /**
   * Count documents matching conditions
   */
  async count(conditions?: QueryCondition[]): Promise<number> {
    await this.ensureLoaded();

    if (!conditions || conditions.length === 0) {
      return Array.from(this.documents.values()).filter((doc) => !doc._deleted).length;
    }

    return Array.from(this.documents.values()).filter(
      (doc) => !doc._deleted && this.matchesConditions(doc.data, conditions)
    ).length;
  }

  // ============================================================================
  // Index Management
  // ============================================================================

  /**
   * Create an index on this collection
   */
  async createIndex(
    name: string,
    fields: string[],
    type: IndexType = 'btree',
    options: { unique?: boolean; sparse?: boolean } = {}
  ): Promise<void> {
    const definition: IndexDefinition = {
      name,
      collection: this.name,
      fields,
      type,
      unique: options.unique,
      sparse: options.sparse,
    };

    await this.indexManager.createIndex(definition);

    // Index all existing documents
    for (const [id, doc] of this.documents) {
      if (!doc._deleted) {
        this.indexManager.indexDocument(this.name, id, doc.data);
      }
    }

    this.database.emit('index:created', { collection: this.name, index: name });
  }

  /**
   * Drop an index
   */
  async dropIndex(name: string): Promise<boolean> {
    const result = await this.indexManager.dropIndex(this.name, name);

    if (result) {
      this.database.emit('index:dropped', { collection: this.name, index: name });
    }

    return result;
  }

  /**
   * Get all indexes on this collection
   */
  getIndexes(): IndexDefinition[] {
    return this.indexManager.getIndexesForCollection(this.name);
  }

  // ============================================================================
  // Collection Management
  // ============================================================================

  /**
   * Get the collection name
   */
  getName(): string {
    return this.name;
  }

  /**
   * Get the parent database
   */
  getDatabase(): Database {
    return this.database;
  }

  /**
   * Load the collection from storage
   */
  async load(): Promise<void> {
    const documents = await this.storage.loadCollection(this.name);
    this.documents.clear();

    for (const doc of documents) {
      this.documents.set(doc._id, doc);

      // Index the document
      if (!doc._deleted) {
        this.indexManager.indexDocument(this.name, doc._id, doc.data);
      }
    }

    this.isLoaded = true;
  }

  /**
   * Flush the collection to storage
   */
  async flush(): Promise<void> {
    await this.storage.flushCollection(this.name);
  }

  /**
   * Drop (delete) the entire collection
   */
  async drop(): Promise<void> {
    // Remove all indexes
    for (const index of this.indexManager.getIndexesForCollection(this.name)) {
      await this.indexManager.dropIndex(this.name, index.name);
    }

    this.documents.clear();
    this.isLoaded = false;
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  private async ensureLoaded(): Promise<void> {
    if (!this.isLoaded) {
      await this.load();
    }
  }

  private tryUseIndex(conditions: QueryCondition[]): DocumentId[] | null {
    // Find an index that can be used for these conditions
    for (const condition of conditions) {
      const index = this.indexManager.findIndexForField(this.name, condition.field);
      if (index) {
        // Use the index to get candidate document IDs
        return this.indexManager.query(
          this.name,
          index.name,
          condition.operator,
          condition.value
        );
      }
    }
    return null;
  }

  private matchesConditions(data: DocumentData, conditions: QueryCondition[]): boolean {
    for (let i = 0; i < conditions.length; i++) {
      const condition = conditions[i];
      const value = this.getNestedValue(data, condition.field);
      const matches = this.evaluateCondition(value, condition.operator, condition.value);

      if (i === 0) {
        if (!matches) return false;
      } else {
        const logic = condition.logic || 'AND';
        if (logic === 'AND' && !matches) return false;
        if (logic === 'OR' && matches) return true;
      }
    }
    return true;
  }

  private evaluateCondition(fieldValue: any, operator: string, conditionValue: any): boolean {
    switch (operator) {
      case '=':
        return fieldValue === conditionValue;
      case '!=':
        return fieldValue !== conditionValue;
      case '>':
        return fieldValue > conditionValue;
      case '>=':
        return fieldValue >= conditionValue;
      case '<':
        return fieldValue < conditionValue;
      case '<=':
        return fieldValue <= conditionValue;
      case 'LIKE':
        return this.matchLike(String(fieldValue || ''), String(conditionValue));
      case 'NOT LIKE':
        return !this.matchLike(String(fieldValue || ''), String(conditionValue));
      case 'IN':
        return Array.isArray(conditionValue) && conditionValue.includes(fieldValue);
      case 'NOT IN':
        return Array.isArray(conditionValue) && !conditionValue.includes(fieldValue);
      case 'BETWEEN':
        return (
          Array.isArray(conditionValue) &&
          conditionValue.length === 2 &&
          fieldValue >= conditionValue[0] &&
          fieldValue <= conditionValue[1]
        );
      case 'IS NULL':
        return fieldValue === null || fieldValue === undefined;
      case 'IS NOT NULL':
        return fieldValue !== null && fieldValue !== undefined;
      case 'CONTAINS':
        return String(fieldValue || '').includes(String(conditionValue));
      case 'STARTS WITH':
        return String(fieldValue || '').startsWith(String(conditionValue));
      case 'ENDS WITH':
        return String(fieldValue || '').endsWith(String(conditionValue));
      case 'MATCHES':
        try {
          return new RegExp(conditionValue).test(String(fieldValue || ''));
        } catch {
          return false;
        }
      default:
        return false;
    }
  }

  private matchLike(value: string, pattern: string): boolean {
    // Convert SQL LIKE pattern to regex
    const regexPattern = pattern
      .replace(/[.+^${}()|[\]\\]/g, '\\$&') // Escape special chars
      .replace(/%/g, '.*') // % = any characters
      .replace(/_/g, '.'); // _ = single character

    return new RegExp(`^${regexPattern}$`, 'i').test(value);
  }

  private getNestedValue(obj: any, path: string): any {
    const parts = path.split('.');
    let current = obj;

    for (const part of parts) {
      if (current === null || current === undefined) {
        return undefined;
      }
      current = current[part];
    }

    return current;
  }

  private sortDocuments(
    documents: StoredDocument[],
    orderBy: OrderByClause[]
  ): StoredDocument[] {
    return [...documents].sort((a, b) => {
      for (const clause of orderBy) {
        const aVal = this.getNestedValue(a.data, clause.field);
        const bVal = this.getNestedValue(b.data, clause.field);

        let comparison = 0;
        if (aVal < bVal) comparison = -1;
        else if (aVal > bVal) comparison = 1;

        if (comparison !== 0) {
          return clause.direction === 'DESC' ? -comparison : comparison;
        }
      }
      return 0;
    });
  }

  private projectFields(data: DocumentData, fields: string[]): DocumentData {
    const result: DocumentData = {};
    for (const field of fields) {
      const value = this.getNestedValue(data, field);
      if (value !== undefined) {
        this.setNestedValue(result, field, value);
      }
    }
    return result;
  }

  private setNestedValue(obj: any, path: string, value: any): void {
    const parts = path.split('.');
    let current = obj;

    for (let i = 0; i < parts.length - 1; i++) {
      if (!(parts[i] in current)) {
        current[parts[i]] = {};
      }
      current = current[parts[i]];
    }

    current[parts[parts.length - 1]] = value;
  }
}
