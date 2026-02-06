/**
 * Main Application Entry Point
 * Initializes and coordinates all modules
 */

import { TodoManager } from './todoManager.js';
import { UI } from './ui.js';

/**
 * Application class - Coordinates the todo application
 */
class App {
    constructor() {
        console.log('%c[TODO COMMAND CENTER]', 'color: #00ffcc; font-weight: bold; font-size: 14px;');
        console.log('%cSystem initializing...', 'color: #7a8a7a;');

        // Initialize manager
        this.todoManager = new TodoManager();

        // Initialize UI with manager
        this.ui = new UI(this.todoManager);

        // Subscribe UI to manager changes
        this.todoManager.subscribe(() => {
            this.ui.render();
        });
    }

    /**
     * Initialize the application
     */
    init() {
        // Setup UI event listeners
        this.ui.init();

        // Initial render
        this.ui.render();

        // Log statistics
        const stats = this.todoManager.getStats();
        console.log(`%cSystem ready. ${stats.pending} active tasks, ${stats.completed} completed.`,
            'color: #00ffcc;');
    }
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new App();
    app.init();
});
