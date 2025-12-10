/**
 * TDB+ NQL Parser (Natural Query Language)
 *
 * A human-friendly, natural language-inspired query language.
 * Makes database queries accessible to everyone.
 *
 * Examples:
 *   find all users where age is greater than 21
 *   get users with name containing "John" sorted by age descending
 *   count orders where status equals "pending"
 *   show first 10 products ordered by price ascending
 *   find users where age is between 18 and 30
 *   get posts where tags include "javascript"
 *   remove users where inactive is true
 *   update users set status to "active" where verified is true
 *   add to users name "Jane", email "jane@example.com", age 28
 */

import {
  ParsedQuery,
  QueryCondition,
  OrderByClause,
  ComparisonOperator,
  QuerySyntaxError,
} from '../../types';

export class NQLParser {
  private input: string;
  private words: string[];
  private position: number;

  constructor() {
    this.input = '';
    this.words = [];
    this.position = 0;
  }

  /**
   * Parse a natural language query
   */
  parse(query: string): ParsedQuery {
    this.input = query.trim().toLowerCase();
    this.words = this.tokenize(query);
    this.position = 0;

    if (this.words.length === 0) {
      throw new QuerySyntaxError('Empty query');
    }

    const firstWord = this.words[0].toLowerCase();

    // Determine query type from first word(s)
    switch (firstWord) {
      case 'find':
      case 'get':
      case 'show':
      case 'select':
      case 'fetch':
      case 'retrieve':
      case 'list':
        return this.parseFind();

      case 'count':
      case 'how':
        return this.parseCount();

      case 'add':
      case 'insert':
      case 'create':
        return this.parseAdd();

      case 'update':
      case 'modify':
      case 'change':
      case 'set':
        return this.parseUpdate();

      case 'remove':
      case 'delete':
      case 'drop':
        return this.parseDelete();

      case 'explain':
        return this.parseExplain();

      default:
        // Try to infer intent
        if (this.input.includes('from') || this.input.includes('in')) {
          return this.parseFind();
        }
        throw new QuerySyntaxError(
          `I don't understand "${firstWord}". Try starting with: find, get, count, add, update, or remove`,
          0,
          'Example: find all users where age is greater than 21'
        );
    }
  }

  // ============================================================================
  // Query Parsers
  // ============================================================================

  private parseFind(): ParsedQuery {
    this.position = 1; // Skip first word

    // Skip optional words
    this.skipWords(['all', 'the', 'every', 'each']);

    // Check for "first N" or "top N"
    let limit: number | undefined;
    if (this.matchWord('first') || this.matchWord('top')) {
      const numWord = this.peekWord();
      if (numWord && /^\d+$/.test(numWord)) {
        limit = parseInt(this.consumeWord(), 10);
      }
    }

    // Get collection name
    const collection = this.parseCollectionName();

    const result: ParsedQuery = {
      type: 'SELECT',
      collection,
      limit,
    };

    // Parse remaining clauses
    this.parseOptionalClauses(result);

    return result;
  }

  private parseCount(): ParsedQuery {
    this.position = 1;

    // Handle "how many"
    if (this.words[0].toLowerCase() === 'how') {
      this.matchWord('many');
    }

    // Skip optional words
    this.skipWords(['all', 'the']);

    const collection = this.parseCollectionName();

    const result: ParsedQuery = {
      type: 'COUNT',
      collection,
    };

    this.parseOptionalClauses(result);

    return result;
  }

  private parseAdd(): ParsedQuery {
    this.position = 1;

    // Skip "to" if present
    this.matchWord('to');

    // Get collection name
    const collection = this.parseCollectionName();

    // Parse field-value pairs
    const data: Record<string, any> = {};

    // Look for field: value patterns
    while (this.position < this.words.length) {
      const field = this.consumeWord();
      if (!field) break;

      // Skip connecting words
      if (['with', 'and', 'having', ','].includes(field.toLowerCase())) {
        continue;
      }

      // Get value
      let value: any;
      if (this.matchWord('is') || this.matchWord('=') || this.matchWord(':')) {
        value = this.parseValue();
      } else {
        value = this.parseValue();
      }

      if (value !== undefined) {
        data[field] = value;
      }
    }

    return {
      type: 'INSERT',
      collection,
      data,
    };
  }

  private parseUpdate(): ParsedQuery {
    this.position = 1;

    // Get collection name
    const collection = this.parseCollectionName();

    // Skip "set" if present
    this.matchWord('set');

    // Parse set clauses
    const data: Record<string, any> = {};

    while (this.position < this.words.length) {
      const word = this.peekWord()?.toLowerCase();
      if (word === 'where' || word === 'if' || word === 'when') break;

      const field = this.consumeWord();
      if (!field) break;

      // Skip connecting words
      if (['and', ','].includes(field.toLowerCase())) {
        continue;
      }

      // Skip "to" or "=" or ":"
      this.matchWord('to') || this.matchWord('=') || this.matchWord(':');

      const value = this.parseValue();
      if (value !== undefined) {
        data[field] = value;
      }
    }

    const result: ParsedQuery = {
      type: 'UPDATE',
      collection,
      data,
    };

    this.parseOptionalClauses(result);

    return result;
  }

  private parseDelete(): ParsedQuery {
    this.position = 1;

    // Skip optional words
    this.skipWords(['all', 'the', 'from']);

    const collection = this.parseCollectionName();

    const result: ParsedQuery = {
      type: 'DELETE',
      collection,
    };

    this.parseOptionalClauses(result);

    return result;
  }

  private parseExplain(): ParsedQuery {
    this.position = 1;
    return { type: 'EXPLAIN', collection: '' };
  }

  // ============================================================================
  // Clause Parsers
  // ============================================================================

  private parseOptionalClauses(result: ParsedQuery): void {
    while (this.position < this.words.length) {
      const word = this.peekWord()?.toLowerCase();

      if (word === 'where' || word === 'with' || word === 'if' || word === 'when' || word === 'having') {
        this.position++;
        if (!result.conditions) {
          result.conditions = [];
        }
        result.conditions.push(...this.parseConditions());
      } else if (word === 'sorted' || word === 'ordered' || word === 'order') {
        this.position++;
        this.matchWord('by');
        result.orderBy = this.parseOrderBy();
      } else if (word === 'limit' || word === 'first' || word === 'top') {
        this.position++;
        const numWord = this.peekWord();
        if (numWord && /^\d+$/.test(numWord)) {
          result.limit = parseInt(this.consumeWord(), 10);
        }
      } else if (word === 'skip' || word === 'offset') {
        this.position++;
        const numWord = this.peekWord();
        if (numWord && /^\d+$/.test(numWord)) {
          result.offset = parseInt(this.consumeWord(), 10);
        }
      } else {
        // Unknown word, try to parse as condition if we haven't started yet
        if (!result.conditions && this.looksLikeCondition()) {
          result.conditions = this.parseConditions();
        } else {
          this.position++;
        }
      }
    }
  }

  private parseConditions(): QueryCondition[] {
    const conditions: QueryCondition[] = [];

    while (this.position < this.words.length) {
      const word = this.peekWord()?.toLowerCase();

      // Stop at ordering/limit keywords
      if (['sorted', 'ordered', 'order', 'limit', 'first', 'top', 'skip', 'offset'].includes(word || '')) {
        break;
      }

      // Skip connecting words at start
      if (word === 'and' || word === ',') {
        this.position++;
        continue;
      }

      if (word === 'or') {
        this.position++;
        // Parse next condition with OR logic
        const condition = this.parseSingleCondition();
        if (condition) {
          condition.logic = 'OR';
          conditions.push(condition);
        }
        continue;
      }

      const condition = this.parseSingleCondition();
      if (condition) {
        conditions.push(condition);
      } else {
        break;
      }
    }

    return conditions;
  }

  private parseSingleCondition(): QueryCondition | null {
    const field = this.consumeWord();
    if (!field) return null;

    // Skip optional "is" or "are"
    this.matchWord('is') || this.matchWord('are');

    // Determine operator from natural language
    const operatorInfo = this.parseNaturalOperator();

    return {
      field,
      operator: operatorInfo.operator,
      value: operatorInfo.value,
    };
  }

  private parseNaturalOperator(): { operator: ComparisonOperator; value: any } {
    const word = this.peekWord()?.toLowerCase();

    // "not" prefix
    if (word === 'not') {
      this.position++;
      const inner = this.parseNaturalOperator();

      // Convert to negative operator
      switch (inner.operator) {
        case '=':
          return { operator: '!=', value: inner.value };
        case 'LIKE':
          return { operator: 'NOT LIKE', value: inner.value };
        case 'IN':
          return { operator: 'NOT IN', value: inner.value };
        case 'IS NULL':
          return { operator: 'IS NOT NULL', value: null };
        default:
          return inner;
      }
    }

    // "greater than" / "more than"
    if (word === 'greater' || word === 'more' || word === 'above' || word === 'over') {
      this.position++;
      this.matchWord('than') || this.matchWord('or');
      if (this.matchWord('equal') || this.matchWord('equals')) {
        this.matchWord('to');
        return { operator: '>=', value: this.parseValue() };
      }
      return { operator: '>', value: this.parseValue() };
    }

    // "less than" / "fewer than"
    if (word === 'less' || word === 'fewer' || word === 'below' || word === 'under') {
      this.position++;
      this.matchWord('than') || this.matchWord('or');
      if (this.matchWord('equal') || this.matchWord('equals')) {
        this.matchWord('to');
        return { operator: '<=', value: this.parseValue() };
      }
      return { operator: '<', value: this.parseValue() };
    }

    // "between"
    if (word === 'between') {
      this.position++;
      const low = this.parseValue();
      this.matchWord('and');
      const high = this.parseValue();
      return { operator: 'BETWEEN', value: [low, high] };
    }

    // "contains" / "containing" / "includes" / "including"
    if (word === 'contains' || word === 'containing' || word === 'includes' || word === 'including' || word === 'has') {
      this.position++;
      return { operator: 'CONTAINS', value: this.parseValue() };
    }

    // "starts with" / "starting with"
    if (word === 'starts' || word === 'starting' || word === 'begins' || word === 'beginning') {
      this.position++;
      this.matchWord('with');
      return { operator: 'STARTS WITH', value: this.parseValue() };
    }

    // "ends with" / "ending with"
    if (word === 'ends' || word === 'ending') {
      this.position++;
      this.matchWord('with');
      return { operator: 'ENDS WITH', value: this.parseValue() };
    }

    // "matches" (regex)
    if (word === 'matches' || word === 'matching') {
      this.position++;
      return { operator: 'MATCHES', value: this.parseValue() };
    }

    // "like"
    if (word === 'like') {
      this.position++;
      return { operator: 'LIKE', value: this.parseValue() };
    }

    // "in"
    if (word === 'in' || word === 'one' || word === 'among') {
      this.position++;
      this.matchWord('of');
      const values = this.parseValueList();
      return { operator: 'IN', value: values };
    }

    // "null" / "empty"
    if (word === 'null' || word === 'empty' || word === 'missing') {
      this.position++;
      return { operator: 'IS NULL', value: null };
    }

    // "equal" / "equals" / "="
    if (word === 'equal' || word === 'equals' || word === '=') {
      this.position++;
      this.matchWord('to');
      return { operator: '=', value: this.parseValue() };
    }

    // "different" / "differs"
    if (word === 'different' || word === 'differs') {
      this.position++;
      this.matchWord('from');
      return { operator: '!=', value: this.parseValue() };
    }

    // Comparison symbols
    if (word === '>' || word === 'gt') {
      this.position++;
      return { operator: '>', value: this.parseValue() };
    }
    if (word === '>=' || word === 'gte') {
      this.position++;
      return { operator: '>=', value: this.parseValue() };
    }
    if (word === '<' || word === 'lt') {
      this.position++;
      return { operator: '<', value: this.parseValue() };
    }
    if (word === '<=' || word === 'lte') {
      this.position++;
      return { operator: '<=', value: this.parseValue() };
    }
    if (word === '!=' || word === '<>' || word === 'ne') {
      this.position++;
      return { operator: '!=', value: this.parseValue() };
    }

    // Default: equality
    return { operator: '=', value: this.parseValue() };
  }

  private parseOrderBy(): OrderByClause[] {
    const clauses: OrderByClause[] = [];

    while (this.position < this.words.length) {
      const word = this.peekWord()?.toLowerCase();

      if (['limit', 'first', 'top', 'skip', 'offset', 'where', 'and'].includes(word || '')) {
        break;
      }

      if (word === ',') {
        this.position++;
        continue;
      }

      const field = this.consumeWord();
      if (!field) break;

      let direction: 'ASC' | 'DESC' = 'ASC';

      const nextWord = this.peekWord()?.toLowerCase();
      if (nextWord === 'desc' || nextWord === 'descending' || nextWord === 'down') {
        direction = 'DESC';
        this.position++;
      } else if (nextWord === 'asc' || nextWord === 'ascending' || nextWord === 'up') {
        direction = 'ASC';
        this.position++;
      }

      clauses.push({ field, direction });
    }

    return clauses;
  }

  // ============================================================================
  // Value Parsing
  // ============================================================================

  private parseValue(): any {
    const word = this.consumeWord();
    if (!word) return undefined;

    // Remove quotes if present
    if ((word.startsWith('"') && word.endsWith('"')) ||
        (word.startsWith("'") && word.endsWith("'"))) {
      return word.slice(1, -1);
    }

    // Boolean
    if (word.toLowerCase() === 'true' || word.toLowerCase() === 'yes') return true;
    if (word.toLowerCase() === 'false' || word.toLowerCase() === 'no') return false;

    // Null
    if (word.toLowerCase() === 'null' || word.toLowerCase() === 'none') return null;

    // Number
    if (/^-?\d+(\.\d+)?$/.test(word)) {
      return parseFloat(word);
    }

    // String (unquoted)
    return word;
  }

  private parseValueList(): any[] {
    const values: any[] = [];

    // Handle parentheses
    this.matchWord('(');

    while (this.position < this.words.length) {
      const word = this.peekWord()?.toLowerCase();

      if (word === ')' || word === 'and' && this.words[this.position + 1]?.toLowerCase() !== ',') {
        break;
      }

      if (word === ',' || word === 'or') {
        this.position++;
        continue;
      }

      const value = this.parseValue();
      if (value !== undefined) {
        values.push(value);
      } else {
        break;
      }
    }

    this.matchWord(')');
    return values;
  }

  // ============================================================================
  // Collection Name Parsing
  // ============================================================================

  private parseCollectionName(): string {
    // Skip articles
    this.skipWords(['from', 'in', 'into']);

    const word = this.consumeWord();
    if (!word) {
      throw new QuerySyntaxError('Expected collection name');
    }

    // Handle plural forms - try to singularize common patterns
    return this.normalizeCollectionName(word);
  }

  private normalizeCollectionName(name: string): string {
    // Remove common suffixes but keep the original if it exists
    // This is a simple heuristic - collections might use plural names
    return name;
  }

  // ============================================================================
  // Tokenizer
  // ============================================================================

  private tokenize(input: string): string[] {
    const words: string[] = [];
    let current = '';
    let inQuote = false;
    let quoteChar = '';

    for (let i = 0; i < input.length; i++) {
      const char = input[i];

      if (inQuote) {
        current += char;
        if (char === quoteChar) {
          inQuote = false;
          words.push(current);
          current = '';
        }
        continue;
      }

      if (char === '"' || char === "'") {
        if (current) {
          words.push(current);
          current = '';
        }
        inQuote = true;
        quoteChar = char;
        current = char;
        continue;
      }

      if (/\s/.test(char) || char === ',') {
        if (current) {
          words.push(current);
          current = '';
        }
        if (char === ',') {
          words.push(',');
        }
        continue;
      }

      // Handle operators as separate tokens
      if (['>', '<', '=', '!'].includes(char)) {
        if (current) {
          words.push(current);
          current = '';
        }
        // Check for two-char operators
        if (i + 1 < input.length && ['>=', '<=', '!=', '<>'].includes(char + input[i + 1])) {
          words.push(char + input[i + 1]);
          i++;
        } else {
          words.push(char);
        }
        continue;
      }

      if (char === '(' || char === ')') {
        if (current) {
          words.push(current);
          current = '';
        }
        words.push(char);
        continue;
      }

      current += char;
    }

    if (current) {
      words.push(current);
    }

    return words;
  }

  // ============================================================================
  // Helpers
  // ============================================================================

  private peekWord(): string | undefined {
    return this.words[this.position];
  }

  private consumeWord(): string {
    return this.words[this.position++] || '';
  }

  private matchWord(word: string): boolean {
    if (this.words[this.position]?.toLowerCase() === word.toLowerCase()) {
      this.position++;
      return true;
    }
    return false;
  }

  private skipWords(words: string[]): void {
    while (this.position < this.words.length) {
      if (words.includes(this.words[this.position].toLowerCase())) {
        this.position++;
      } else {
        break;
      }
    }
  }

  private looksLikeCondition(): boolean {
    // Check if current position looks like start of a condition
    const remaining = this.words.slice(this.position, this.position + 5).join(' ').toLowerCase();
    const conditionPatterns = [
      /\w+\s+(is|are|equals?|contains?|starts?|ends?|greater|less|between)/,
      /\w+\s*[><=!]/,
    ];
    return conditionPatterns.some((p) => p.test(remaining));
  }
}
