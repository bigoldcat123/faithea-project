/**
 * UI Module - Handles all DOM manipulations and user interactions
 * Manages the visual presentation and user events
 */

export class UI {
    constructor(todoManager) {
        this.todoManager = todoManager;

        // DOM elements
        this.elements = {
            todoInput: document.getElementById('todoInput'),
            addBtn: document.getElementById('addBtn'),
            todoList: document.getElementById('todoList'),
            emptyState: document.getElementById('emptyState'),
            filterBtns: document.querySelectorAll('.filter-btn'),
            priorityBtns: document.querySelectorAll('.priority-btn'),
            pendingCount: document.getElementById('pendingCount'),
            completedCount: document.getElementById('completedCount'),
            clearCompleted: document.getElementById('clearCompleted'),
            timestamp: document.getElementById('timestamp')
        };

        this.selectedPriority = 'medium';
    }

    /**
     * Initialize all UI event listeners
     */
    init() {
        // Add todo button
        this.elements.addBtn.addEventListener('click', () => this.handleAdd());

        // Enter key in input
        this.elements.todoInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.handleAdd();
            }
        });

        // Priority buttons
        this.elements.priorityBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                this.elements.priorityBtns.forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                this.selectedPriority = btn.dataset.priority;
            });
        });

        // Filter buttons
        this.elements.filterBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                this.elements.filterBtns.forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                this.todoManager.setFilter(btn.dataset.filter);
            });
        });

        // Clear completed button
        this.elements.clearCompleted.addEventListener('click', () => {
            this.handleClearCompleted();
        });

        // Update timestamp
        this.updateTimestamp();
        setInterval(() => this.updateTimestamp(), 1000);

        console.log('[UI] Initialized');
    }

    /**
     * Handle adding a new todo
     */
    handleAdd() {
        const text = this.elements.todoInput.value;

        try {
            this.todoManager.add(text, this.selectedPriority);
            this.elements.todoInput.value = '';
            this.elements.todoInput.focus();
        } catch (error) {
            console.error('[UI] Failed to add todo:', error);
            this.shakeInput();
        }
    }

    /**
     * Shake animation for invalid input
     */
    shakeInput() {
        const wrapper = this.elements.todoInput.closest('.input-wrapper');
        wrapper.style.animation = 'shake 0.5s ease-in-out';
        setTimeout(() => {
            wrapper.style.animation = '';
        }, 500);
    }

    /**
     * Handle clearing completed todos
     */
    handleClearCompleted() {
        const clearedCount = this.todoManager.clearCompleted();
        if (clearedCount === 0) {
            console.log('[UI] No completed todos to clear');
        }
    }

    /**
     * Handle todo checkbox toggle
     * @param {string} id - Todo ID
     */
    handleToggle(id) {
        this.todoManager.toggle(id);
    }

    /**
     * Handle todo delete
     * @param {string} id - Todo ID
     */
    handleDelete(id) {
        this.todoManager.delete(id);
    }

    /**
     * Render the todo list
     */
    render() {
        const todos = this.todoManager.getFiltered();
        const stats = this.todoManager.getStats();

        // Update stats
        this.animateValue(this.elements.pendingCount, stats.pending);
        this.animateValue(this.elements.completedCount, stats.completed);

        // Clear list
        this.elements.todoList.innerHTML = '';

        // Show/hide empty state
        if (todos.length === 0) {
            this.elements.emptyState.classList.remove('hidden');
            this.elements.todoList.classList.add('hidden');
        } else {
            this.elements.emptyState.classList.add('hidden');
            this.elements.todoList.classList.remove('hidden');

            // Render todos
            todos.forEach(todo => {
                const todoElement = this.createTodoElement(todo);
                this.elements.todoList.appendChild(todoElement);
            });
        }
    }

    /**
     * Create a todo DOM element
     * @param {Object} todo - Todo object
     * @returns {HTMLElement} Todo list item element
     */
    createTodoElement(todo) {
        const li = document.createElement('li');
        li.className = `todo-item priority-${todo.priority}${todo.completed ? ' completed' : ''}`;
        li.dataset.id = todo.id;

        const date = new Date(todo.createdAt);
        const formattedDate = this.formatDate(date);

        li.innerHTML = `
            <div class="todo-checkbox-wrapper">
                <input
                    type="checkbox"
                    class="todo-checkbox"
                    ${todo.completed ? 'checked' : ''}
                >
            </div>
            <div class="todo-content">
                <div class="todo-text">${this.escapeHtml(todo.text)}</div>
                <div class="todo-meta">
                    <span class="todo-priority ${todo.priority}">${todo.priority}</span>
                    <span class="todo-date">${formattedDate}</span>
                </div>
            </div>
            <button class="todo-delete" title="Delete todo">×</button>
        `;

        // Event listeners
        const checkbox = li.querySelector('.todo-checkbox');
        checkbox.addEventListener('change', () => this.handleToggle(todo.id));

        const deleteBtn = li.querySelector('.todo-delete');
        deleteBtn.addEventListener('click', () => this.handleDelete(todo.id));

        return li;
    }

    /**
     * Animate a number value change
     * @param {HTMLElement} element - Element to animate
     * @param {number} value - New value
     */
    animateValue(element, value) {
        const currentValue = parseInt(element.textContent) || 0;
        if (currentValue === value) return;

        element.style.transform = 'scale(1.2)';
        element.style.opacity = '0.5';

        setTimeout(() => {
            element.textContent = value;
            element.style.transform = 'scale(1)';
            element.style.opacity = '1';
        }, 150);
    }

    /**
     * Format date for display
     * @param {Date} date - Date to format
     * @returns {string} Formatted date string
     */
    formatDate(date) {
        const now = new Date();
        const diff = now - date;

        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(diff / 3600000);
        const days = Math.floor(diff / 86400000);

        if (minutes < 1) return 'Just now';
        if (minutes < 60) return `${minutes}m ago`;
        if (hours < 24) return `${hours}h ago`;
        if (days < 7) return `${days}d ago`;

        return date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric'
        });
    }

    /**
     * Update timestamp display
     */
    updateTimestamp() {
        const now = new Date();
        const time = now.toLocaleTimeString('en-US', {
            hour12: false,
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
        });
        this.elements.timestamp.textContent = time;
    }

    /**
     * Escape HTML to prevent XSS
     * @param {string} text - Text to escape
     * @returns {string} Escaped text
     */
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Add shake animation
const style = document.createElement('style');
style.textContent = `
    @keyframes shake {
        0%, 100% { transform: translateX(0); }
        10%, 30%, 50%, 70%, 90% { transform: translateX(-4px); }
        20%, 40%, 60%, 80% { transform: translateX(4px); }
    }
`;
document.head.appendChild(style);
