// Wyldlands Gateway Admin Dashboard JS

async function fetchStats() {
    try {
        const response = await fetch('/stats');
        if (!response.ok) throw new Error('Failed to fetch stats');
        const stats = await response.json();
        
        document.getElementById('total-sessions').textContent = stats.total_sessions;
        document.getElementById('uptime').textContent = formatUptime(stats.uptime_seconds);
        document.getElementById('memory-usage').textContent = stats.memory_usage_mb ? `${stats.memory_usage_mb.toFixed(2)} MB` : 'N/A';
        
        // Update sidechannel list
        const protocolList = document.getElementById('sidechannel-list');
        protocolList.innerHTML = '';
        for (const [protocol, count] of Object.entries(stats.connections_by_protocol)) {
            const li = document.createElement('li');
            li.textContent = `${protocol}: ${count}`;
            protocolList.appendChild(li);
        }
        
        // Update state distribution
        const stateDist = document.getElementById('state-distribution');
        stateDist.innerHTML = '';
        for (const [state, count] of Object.entries(stats.sessions_by_state)) {
            const div = document.createElement('div');
            div.textContent = `${state}: ${count}`;
            stateDist.appendChild(div);
        }
    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

async function fetchSessions() {
    try {
        const response = await fetch('/sessions');
        if (!response.ok) throw new Error('Failed to fetch sessions');
        const sessions = await response.json();
        
        const list = document.getElementById('sessions-list');
        list.innerHTML = '';
        
        sessions.forEach(session => {
            const row = document.createElement('tr');
            row.innerHTML = `
                <td>${session.id.substring(0, 8)}...</td>
                <td>${session.state}</td>
                <td>${session.protocol}</td>
                <td>${session.client_addr}</td>
                <td>${session.last_activity}</td>
                <td>
                    <button onclick="disconnectSession('${session.id}')" class="btn-danger">Disconnect</button>
                </td>
            `;
            list.appendChild(row);
        });
    } catch (error) {
        console.error('Error fetching sessions:', error);
    }
}

async function checkHealth() {
    try {
        const response = await fetch('/health');
        const healthVal = document.getElementById('health-value');
        if (response.ok) {
            healthVal.textContent = 'OK';
            healthVal.className = 'ok';
        } else {
            healthVal.textContent = 'ERROR';
            healthVal.className = 'error';
        }
    } catch (error) {
        const healthVal = document.getElementById('health-value');
        healthVal.textContent = 'UNREACHABLE';
        healthVal.className = 'error';
    }
}

async function sendBroadcast() {
    const msgInput = document.getElementById('broadcast-msg');
    const message = msgInput.value.trim();
    if (!message) return;
    
    try {
        const response = await fetch('/command', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                command: 'broadcast',
                params: { message: message }
            })
        });
        const result = await response.json();
        alert(result.message);
        if (result.success) msgInput.value = '';
    } catch (error) {
        alert('Failed to send broadcast');
    }
}

async function disconnectSession(sessionId) {
    if (!confirm(`Are you sure you want to disconnect session ${sessionId}?`)) return;
    
    try {
        const response = await fetch('/command', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                command: 'disconnect_session',
                params: { session_id: sessionId }
            })
        });
        const result = await response.json();
        alert(result.message);
        fetchSessions();
    } catch (error) {
        alert('Failed to disconnect session');
    }
}

function formatUptime(seconds) {
    if (seconds === 0) return "Just started";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h}h ${m}m ${s}s`;
}

// Initial fetch
checkHealth();
fetchStats();
fetchSessions();

// Periodic updates
setInterval(checkHealth, 30000);
setInterval(fetchStats, 5000);
setInterval(fetchSessions, 10000);
