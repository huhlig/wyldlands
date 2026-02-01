let socket = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    connect();
    setupInput();
});

function connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${location.host}/websocket`;

    addToConsole('Connecting to server...', 'system');

    socket = new WebSocket(wsUrl);

    socket.onopen = (event) => {
        addToConsole('Connected to Wyldlands!', 'system');
        reconnectAttempts = 0;
        updateConnectionStatus(true);
    };

    socket.onmessage = (event) => {
        addToConsole(event.data, 'server');
    };

    socket.onerror = (error) => {
        console.error('WebSocket error:', error);
        addToConsole('Connection error occurred', 'error');
    };

    socket.onclose = (event) => {
        addToConsole('Disconnected from server', 'system');
        updateConnectionStatus(false);

        // Attempt to reconnect
        if (reconnectAttempts < maxReconnectAttempts) {
            reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
            addToConsole(`Reconnecting in ${delay/1000} seconds... (Attempt ${reconnectAttempts}/${maxReconnectAttempts})`, 'system');
            setTimeout(connect, delay);
        } else {
            addToConsole('Max reconnection attempts reached. Please refresh the page.', 'error');
        }
    };
}

function setupInput() {
    const inputField = document.createElement('input');
    inputField.type = 'text';
    inputField.id = 'command-input';
    inputField.placeholder = 'Enter command...';
    inputField.autocomplete = 'off';

    inputField.addEventListener('keypress', (event) => {
        if (event.key === 'Enter') {
            sendCommand();
        }
    });

    const sendButton = document.createElement('button');
    sendButton.id = 'send-button';
    sendButton.textContent = 'Send';
    sendButton.onclick = sendCommand;

    const footer = document.getElementById('wyldlands-footer');
    footer.innerHTML = '';
    footer.appendChild(inputField);
    footer.appendChild(sendButton);
}

function sendCommand() {
    const input = document.getElementById('command-input');
    const command = input.value.trim();

    if (command && socket && socket.readyState === WebSocket.OPEN) {
        addToConsole(`> ${command}`, 'user');
        socket.send(command);
        input.value = '';
    } else if (!socket || socket.readyState !== WebSocket.OPEN) {
        addToConsole('Not connected to server', 'error');
    }
}

function addToConsole(text, type = 'normal') {
    const console = document.getElementById('wyldlands-console');
    const line = document.createElement('div');
    line.className = `console-line console-${type}`;

    const timestamp = new Date().toLocaleTimeString();
    line.innerHTML = `<span class="timestamp">[${timestamp}]</span> ${escapeHtml(text)}`;

    console.appendChild(line);
    console.scrollTop = console.scrollHeight;

    // Limit console history to 500 lines
    while (console.children.length > 500) {
        console.removeChild(console.firstChild);
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function updateConnectionStatus(connected) {
    const header = document.getElementById('wyldlands-header');
    const statusText = connected ? '● Connected' : '○ Disconnected';
    const statusClass = connected ? 'connected' : 'disconnected';
    header.innerHTML = `<span class="status ${statusClass}">${statusText}</span> Wyldlands MUD Client`;
}