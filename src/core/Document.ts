/**
 * TDB+ Document - Individual Document Wrapper
 *
 * Provides a convenient API for working with individual documents,
 * including automatic saving and event handling.
 */

import { Collection } from './Collection';
import { DocumentId, DocumentData, StoredDocument } from '../types';

export class Document {
  private stored: StoredDocument;
  private collection: Collection;
  private isDirty: boolean;

  constructor(stored: StoredDocument, collection: Collection) {
    this.stored = stored;
    this.collection = collection;
    this.isDirty = false;
  }

  // ============================================================================
  // Properties
  // ============================================================================

  /**
   * Get the document ID
   */
  get id(): DocumentId {
    return this.stored._id;
  }

  /**
   * Get the document revision number
   */
  get revision(): number {
    return this.stored._rev;
  }

  /**
   * Get the creation timestamp
   */
  get createdAt(): Date {
    return this.stored._createdAt;
  }

  /**
   * Get the last update timestamp
   */
  get updatedAt(): Date {
    return this.stored._updatedAt;
  }

  /**
   * Get the raw document data
   */
  get data(): DocumentData {
    return { ...this.stored.data };
  }

  // ============================================================================
  // Data Access
  // ============================================================================

  /**
   * Get a field value from the document
   */
  get<T = any>(field: string): T | undefined {
    return this.getNestedValue(this.stored.data, field);
  }

  /**
   * Set a field value (does not persist until save() is called)
   */
  set(field: string, value: any): this {
    this.setNestedValue(this.stored.data, field, value);
    this.isDirty = true;
    return this;
  }

  /**
   * Check if a field exists
   */
  has(field: string): boolean {
    return this.get(field) !== undefined;
  }

  /**
   * Remove a field (does not persist until save() is called)
   */
  unset(field: string): this {
    const parts = field.split('.');
    let current = this.stored.data;

    for (let i = 0; i < parts.length - 1; i++) {
      if (!(parts[i] in current)) {
        return this;
      }
      current = current[parts[i]];
    }

    delete current[parts[parts.length - 1]];
    this.isDirty = true;
    return this;
  }

  /**
   * Merge data into the document
   */
  merge(data: DocumentData): this {
    Object.assign(this.stored.data, data);
    this.isDirty = true;
    return this;
  }

  // ============================================================================
  // Persistence
  // ============================================================================

  /**
   * Save changes to the database
   */
  async save(): Promise<this> {
    if (this.isDirty) {
      await this.collection.updateById(this.id, this.stored.data);
      this.isDirty = false;
    }
    return this;
  }

  /**
   * Refresh the document from the database
   */
  async refresh(): Promise<this> {
    const fresh = await this.collection.findById(this.id);
    if (fresh) {
      this.stored = (fresh as any).stored;
      this.isDirty = false;
    }
    return this;
  }

  /**
   * Delete this document from the database
   */
  async delete(): Promise<boolean> {
    return this.collection.deleteById(this.id);
  }

  // ============================================================================
  // Conversion
  // ============================================================================

  /**
   * Convert to a plain JavaScript object
   */
  toObject(): DocumentData & { _id: DocumentId } {
    return {
      _id: this.id,
      ...this.stored.data,
    };
  }

  /**
   * Convert to JSON string
   */
  toJSON(): string {
    return JSON.stringify(this.toObject());
  }

  /**
   * Get a string representation
   */
  toString(): string {
    return `Document(${this.id})`;
  }

  // ============================================================================
  // Private Helpers
  // ============================================================================

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
