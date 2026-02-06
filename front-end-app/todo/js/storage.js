/**
 * Storage Module - Handles localStorage operations
 * Provides persistent storage for todos with error handling
 */

const STORAGE_KEY = 'todo_command_center_data';

export const storage = {
    /**
     * Save todos to localStorage
     * @param {Array} todos - Array of todo objects
     */
    save(todos) {
        try {
            const data = JSON.stringify(todos);
            localStorage.setItem(STORAGE_KEY, data);
        } catch (error) {
            console.error('[STORAGE] Failed to save todos:', error);
        }
    },

    /**
     * Load todos from localStorage
     * @returns {Array} Array of todo objects or empty array if none exist
     */
    load() {
        try {
            const data = localStorage.getItem(STORAGE_KEY);
            return data ? JSON.parse(data) : [];
        } catch (error) {
            console.error('[STORAGE] Failed to load todos:', error);
            return [];
        }
    },

    /**
     * Clear all todos from localStorage
     */
    clear() {
        try {
            localStorage.removeItem(STORAGE_KEY);
        } catch (error) {
            console.error('[STORAGE] Failed to clear todos:', error);
        }
    }
};
