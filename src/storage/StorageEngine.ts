/**
 * TDB+ Storage Engine - Abstract Storage Interface
 *
 * Defines the contract for all storage implementations.
 * Supports pluggable storage backends.
 */

import { StorageOptions, StoredDocument, StorageStats, CollectionName } from '../types';

export abstract class StorageEngine {
  protected options: StorageOptions;

  constructor(options: StorageOptions) {
    this.options = options;
  }

  /**
   * Initialize the storage engine
   */
  abstract initialize(): Promise<void>;

  /**
   * Close the storage engine
   */
  abstract close(): Promise<void>;

  /**
   * List all collections
   */
  abstract listCollections(): Promise<CollectionName[]>;

  /**
   * Load all documents from a collection
   */
  abstract loadCollection(name: CollectionName): Promise<StoredDocument[]>;

  /**
   * Save a document
   */
  abstract saveDocument(collection: CollectionName, document: StoredDocument): Promise<void>;

  /**
   * Delete a document
   */
  abstract deleteDocument(collection: CollectionName, documentId: string): Promise<void>;

  /**
   * Drop an entire collection
   */
  abstract dropCollection(name: CollectionName): Promise<void>;

  /**
   * Flush a collection to persistent storage
   */
  abstract flushCollection(name: CollectionName): Promise<void>;

  /**
   * Get storage statistics
   */
  abstract getStats(): Promise<StorageStats>;
}
