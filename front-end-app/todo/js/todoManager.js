/**
 * Todo Manager Module - Core business logic for todo operations
 * Handles CRUD operations and state management
 */

import { storage } from './storage.js';

export class TodoManager {
    constructor() {
        this.todos = storage.load();
        this.filter = 'all'; // all, active, completed
        this.listeners = [];
    }

    /**
     * Subscribe to state changes
     * @param {Function} callback - Function to call when state changes
     */
    subscribe(callback) {
        this.listeners.push(callback);
    }

    /**
     * Notify all subscribers of state change
     */
    notify() {
        this.listeners.forEach(callback => callback(this.todos));
    }

    /**
     * Get all todos
     * @returns {Array} Array of all todos
     */
    getAll() {
        return [...this.todos];
    }

    /**
     * Get filtered todos based on current filter
     * @returns {Array} Filtered array of todos
     */
    getFiltered() {
        switch (this.filter) {
            case 'active':
                return this.todos.filter(todo => !todo.completed);
            case 'completed':
                return this.todos.filter(todo => todo.completed);
            default:
                return [...this.todos];
        }
    }

    /**
     * Get current filter
     * @returns {string} Current filter value
     */
    getFilter() {
        return this.filter;
    }

    /**
     * Set filter and notify listeners
     * @param {string} filter - Filter value (all, active, completed)
     */
    setFilter(filter) {
        this.filter = filter;
        this.notify();
    }

    /**
     * Add a new todo
     * @param {string} text - Todo text content
     * @param {string} priority - Priority level (low, medium, high)
     * @returns {Object} The created todo object
     */
    add(text, priority = 'medium') {
        const trimmedText = text.trim();
        if (!trimmedText) {
            throw new Error('Todo text cannot be empty');
        }

        const todo = {
            id: Date.now().toString(36) + Math.random().toString(36).substr(2),
            text: trimmedText,
            priority,
            completed: false,
            createdAt: new Date().toISOString()
        };

        this.todos.unshift(todo); // Add to beginning
        this.save();
        this.notify();

        console.log('[TODO_MANAGER] Added:', todo);
        return todo;
    }

    /**
     * Toggle todo completion status
     * @param {string} id - Todo ID
     */
    toggle(id) {
        const todo = this.todos.find(t => t.id === id);
        if (todo) {
            todo.completed = !todo.completed;
            this.save();
            this.notify();
            console.log('[TODO_MANAGER] Toggled:', id, todo.completed);
        }
    }

    /**
     * Delete a todo
     * @param {string} id - Todo ID
     */
    delete(id) {
        const index = this.todos.findIndex(t => t.id === id);
        if (index !== -1) {
            this.todos.splice(index, 1);
            this.save();
            this.notify();
            console.log('[TODO_MANAGER] Deleted:', id);
        }
    }

    /**
     * Clear all completed todos
     * @returns {number} Number of todos cleared
     */
    clearCompleted() {
        const beforeLength = this.todos.length;
        this.todos = this.todos.filter(todo => !todo.completed);
        const clearedCount = beforeLength - this.todos.length;

        if (clearedCount > 0) {
            this.save();
            this.notify();
            console.log('[TODO_MANAGER] Cleared completed todos:', clearedCount);
        }

        return clearedCount;
    }

    /**
     * Get statistics
     * @returns {Object} Statistics object with pending and completed counts
     */
    getStats() {
        const pending = this.todos.filter(t => !t.completed).length;
        const completed = this.todos.filter(t => t.completed).length;
        return { pending, completed };
    }

    /**
     * Save todos to storage
     */
    save() {
        storage.save(this.todos);
    }
}
