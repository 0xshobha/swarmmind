/**
 * SwarmMind Dashboard — Real-time Swarm Visualization
 *
 * Simulates a live SwarmMind network for demonstration.
 * In production, this would connect via WebSocket to running agents.
 */

// ═══════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════
const AGENTS = [
    { id: 'agent-alpha', name: 'Alpha', caps: ['compute', 'reasoning'], color: '#06d6a0', emoji: 'α' },
    { id: 'agent-beta', name: 'Beta', caps: ['data-analysis', 'natural-language'], color: '#4cc9f0', emoji: 'β' },
    { id: 'agent-gamma', name: 'Gamma', caps: ['code-generation', 'reasoning'], color: '#7b2ff7', emoji: 'γ' },
    { id: 'agent-delta', name: 'Delta', caps: ['image-processing', 'compute'], color: '#f72585', emoji: 'δ' },
    { id: 'agent-epsilon', name: 'Epsilon', caps: ['natural-language', 'data-analysis'], color: '#ff6b35', emoji: 'ε' },
    { id: 'agent-zeta', name: 'Zeta', caps: ['compute', 'code-generation'], color: '#ffd166', emoji: 'ζ' },
    { id: 'agent-eta', name: 'Eta', caps: ['reasoning', 'image-processing'], color: '#a8dadc', emoji: 'η' },
    { id: 'agent-theta', name: 'Theta', caps: ['data-analysis', 'compute'], color: '#e76f51', emoji: 'θ' },
];

const TASK_POOL = [
    { desc: 'Analyze anomaly patterns in sensor data', caps: ['data-analysis'], complexity: 4 },
    { desc: 'Generate NLP summary of fleet logs', caps: ['natural-language'], complexity: 3 },
    { desc: 'Run ML inference on drone telemetry', caps: ['compute', 'reasoning'], complexity: 7 },
    { desc: 'Process satellite imagery for terrain mapping', caps: ['image-processing'], complexity: 6 },
    { desc: 'Optimize swarm pathfinding algorithm', caps: ['code-generation'], complexity: 8 },
    { desc: 'Classify objects from LIDAR point cloud', caps: ['compute'], complexity: 5 },
    { desc: 'Translate mission parameters to ROS commands', caps: ['code-generation'], complexity: 4 },
    { desc: 'Generate coordination report', caps: ['natural-language', 'data-analysis'], complexity: 3 },
    { desc: 'Train federated learning model shard', caps: ['compute', 'reasoning'], complexity: 9 },
    { desc: 'Validate consensus proof chain', caps: ['reasoning'], complexity: 5 },
];

// ═══════════════════════════════════════════
// State
// ═══════════════════════════════════════════
const state = {
    agents: new Map(),
    tasks: new Map(),
    events: [],
    taskCounter: 0,
    totalTasksProcessed: 0,
    auctionsWon: 0,
    latencies: [],
};

// Initialize agents
AGENTS.forEach((a, i) => {
    state.agents.set(a.id, {
        ...a,
        status: 'offline',
        role: 'worker',
        reputation: 100,
        activeTasks: 0,
        lastSeen: 0,
        x: 0, y: 0, // topology position
        targetX: 0, targetY: 0,
        vx: 0, vy: 0,
        joined: false,
    });
});

// ═══════════════════════════════════════════
// Clock
// ═══════════════════════════════════════════
function updateClock() {
    const now = new Date();
    document.getElementById('clock').textContent =
        now.toLocaleTimeString('en-US', { hour12: false });
}
setInterval(updateClock, 1000);
updateClock();

// ═══════════════════════════════════════════
// Background Particles
// ═══════════════════════════════════════════
const particleCanvas = document.getElementById('particle-canvas');
const pCtx = particleCanvas.getContext('2d');
let particles = [];

function resizeParticleCanvas() {
    particleCanvas.width = window.innerWidth;
    particleCanvas.height = window.innerHeight;
}
window.addEventListener('resize', resizeParticleCanvas);
resizeParticleCanvas();

function initParticles() {
    particles = [];
    for (let i = 0; i < 60; i++) {
        particles.push({
            x: Math.random() * particleCanvas.width,
            y: Math.random() * particleCanvas.height,
            vx: (Math.random() - 0.5) * 0.3,
            vy: (Math.random() - 0.5) * 0.3,
            r: Math.random() * 2 + 0.5,
            alpha: Math.random() * 0.3 + 0.1,
        });
    }
}
initParticles();

function drawParticles() {
    pCtx.clearRect(0, 0, particleCanvas.width, particleCanvas.height);

    for (const p of particles) {
        p.x += p.vx;
        p.y += p.vy;
        if (p.x < 0) p.x = particleCanvas.width;
        if (p.x > particleCanvas.width) p.x = 0;
        if (p.y < 0) p.y = particleCanvas.height;
        if (p.y > particleCanvas.height) p.y = 0;

        pCtx.beginPath();
        pCtx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        pCtx.fillStyle = `rgba(6, 214, 160, ${p.alpha})`;
        pCtx.fill();
    }

    // Draw connections
    for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
            const dx = particles[i].x - particles[j].x;
            const dy = particles[i].y - particles[j].y;
            const dist = Math.sqrt(dx * dx + dy * dy);
            if (dist < 120) {
                pCtx.beginPath();
                pCtx.moveTo(particles[i].x, particles[i].y);
                pCtx.lineTo(particles[j].x, particles[j].y);
                pCtx.strokeStyle = `rgba(6, 214, 160, ${0.05 * (1 - dist / 120)})`;
                pCtx.lineWidth = 0.5;
                pCtx.stroke();
            }
        }
    }

    requestAnimationFrame(drawParticles);
}
drawParticles();

// ═══════════════════════════════════════════
// Topology Visualization
// ═══════════════════════════════════════════
const topoCanvas = document.getElementById('topology-canvas');
const tCtx = topoCanvas.getContext('2d');
let topoWidth, topoHeight;

function resizeTopoCanvas() {
    const rect = topoCanvas.parentElement.getBoundingClientRect();
    topoCanvas.width = rect.width;
    topoCanvas.height = 400;
    topoWidth = topoCanvas.width;
    topoHeight = topoCanvas.height;

    // Re-position agents in circular layout
    const cx = topoWidth / 2;
    const cy = topoHeight / 2;
    const radius = Math.min(topoWidth, topoHeight) * 0.32;

    let i = 0;
    state.agents.forEach((agent) => {
        const angle = (i / state.agents.size) * Math.PI * 2 - Math.PI / 2;
        agent.targetX = cx + Math.cos(angle) * radius;
        agent.targetY = cy + Math.sin(angle) * radius;
        if (!agent.joined) {
            agent.x = cx;
            agent.y = cy;
        }
        i++;
    });
}
window.addEventListener('resize', resizeTopoCanvas);
setTimeout(resizeTopoCanvas, 100);

function drawTopology() {
    tCtx.clearRect(0, 0, topoWidth, topoHeight);

    const activeAgents = [...state.agents.values()].filter(a => a.status !== 'offline');

    // Draw connections between active agents
    for (let i = 0; i < activeAgents.length; i++) {
        for (let j = i + 1; j < activeAgents.length; j++) {
            const a = activeAgents[i];
            const b = activeAgents[j];

            const gradient = tCtx.createLinearGradient(a.x, a.y, b.x, b.y);
            gradient.addColorStop(0, a.color + '40');
            gradient.addColorStop(1, b.color + '40');

            tCtx.beginPath();
            tCtx.moveTo(a.x, a.y);
            tCtx.lineTo(b.x, b.y);
            tCtx.strokeStyle = gradient;
            tCtx.lineWidth = 1.5;
            tCtx.stroke();

            // Animated data packet
            const t = (Date.now() % 3000) / 3000;
            const packetX = a.x + (b.x - a.x) * t;
            const packetY = a.y + (b.y - a.y) * t;
            tCtx.beginPath();
            tCtx.arc(packetX, packetY, 2, 0, Math.PI * 2);
            tCtx.fillStyle = '#06d6a0';
            tCtx.fill();
        }
    }

    // Draw agents
    state.agents.forEach((agent) => {
        // Smooth movement
        agent.x += (agent.targetX - agent.x) * 0.05;
        agent.y += (agent.targetY - agent.y) * 0.05;

        const isActive = agent.status !== 'offline';
        const alpha = isActive ? 1 : 0.2;

        // Glow
        if (isActive) {
            const glowRadius = 25 + Math.sin(Date.now() / 500) * 5;
            const glow = tCtx.createRadialGradient(agent.x, agent.y, 0, agent.x, agent.y, glowRadius);
            glow.addColorStop(0, agent.color + '30');
            glow.addColorStop(1, 'transparent');
            tCtx.beginPath();
            tCtx.arc(agent.x, agent.y, glowRadius, 0, Math.PI * 2);
            tCtx.fillStyle = glow;
            tCtx.fill();
        }

        // Node circle
        tCtx.beginPath();
        tCtx.arc(agent.x, agent.y, 16, 0, Math.PI * 2);
        tCtx.fillStyle = isActive ? agent.color : '#2a2a3a';
        tCtx.globalAlpha = alpha;
        tCtx.fill();
        tCtx.globalAlpha = 1;

        // Status ring
        if (agent.status === 'stale') {
            tCtx.beginPath();
            tCtx.arc(agent.x, agent.y, 19, 0, Math.PI * 2);
            tCtx.strokeStyle = '#f72585';
            tCtx.lineWidth = 2;
            tCtx.setLineDash([4, 4]);
            tCtx.stroke();
            tCtx.setLineDash([]);
        }

        // Letter
        tCtx.fillStyle = isActive ? '#0a0e1a' : '#4a5568';
        tCtx.font = 'bold 14px Inter';
        tCtx.textAlign = 'center';
        tCtx.textBaseline = 'middle';
        tCtx.fillText(agent.emoji, agent.x, agent.y + 1);

        // Name label
        tCtx.fillStyle = isActive ? '#e8ecf4' : '#4a5568';
        tCtx.font = '11px Inter';
        tCtx.fillText(agent.name, agent.x, agent.y + 30);

        // Rep score
        if (isActive) {
            tCtx.fillStyle = '#ffd166';
            tCtx.font = '10px JetBrains Mono';
            tCtx.fillText(`★${agent.reputation}`, agent.x, agent.y + 42);
        }
    });

    // Center label
    tCtx.fillStyle = '#4a5568';
    tCtx.font = '10px JetBrains Mono';
    tCtx.textAlign = 'center';
    tCtx.fillText('VERTEX BFT MESH', topoWidth / 2, topoHeight / 2 - 10);
    tCtx.fillText(`${activeAgents.length} NODES`, topoWidth / 2, topoHeight / 2 + 5);

    requestAnimationFrame(drawTopology);
}
drawTopology();

// ═══════════════════════════════════════════
// Event Logging
// ═══════════════════════════════════════════
function addLog(icon, msg, cls = '') {
    const log = document.getElementById('event-log');
    const time = new Date().toLocaleTimeString('en-US', { hour12: false });

    const entry = document.createElement('div');
    entry.className = 'log-entry';
    entry.innerHTML = `
        <span class="log-time">${time}</span>
        <span class="log-icon">${icon}</span>
        <span class="log-msg ${cls}">${msg}</span>
    `;

    log.insertBefore(entry, log.firstChild);

    // Limit log entries
    while (log.children.length > 100) {
        log.removeChild(log.lastChild);
    }
}

document.getElementById('clear-log').addEventListener('click', () => {
    document.getElementById('event-log').innerHTML = '';
});

// ═══════════════════════════════════════════
// Agent Card Rendering
// ═══════════════════════════════════════════
function renderAgents() {
    const list = document.getElementById('agents-list');
    list.innerHTML = '';

    state.agents.forEach((agent) => {
        if (!agent.joined) return;

        const statusClass = `status-${agent.status}`;
        const capsHtml = agent.caps
            .map(c => `<span class="cap-badge">${c}</span>`)
            .join('');

        const card = document.createElement('div');
        card.className = 'agent-card';
        card.innerHTML = `
            <div class="agent-avatar" style="background: ${agent.color}20; border: 1px solid ${agent.color}40;">
                <span style="color: ${agent.color}">${agent.emoji}</span>
            </div>
            <div class="agent-info">
                <div class="agent-name">${agent.name}</div>
                <div class="agent-role">${agent.role} · ${agent.activeTasks} tasks</div>
                <div class="agent-caps">${capsHtml}</div>
            </div>
            <div class="agent-stats">
                <span class="agent-status-badge ${statusClass}">${agent.status.toUpperCase()}</span>
                <span class="agent-rep">★ ${agent.reputation}</span>
            </div>
        `;
        list.appendChild(card);
    });
}

// ═══════════════════════════════════════════
// KPI Updates
// ═══════════════════════════════════════════
function updateKPIs() {
    const activeCount = [...state.agents.values()].filter(a => a.status === 'active' || a.status === 'busy').length;
    document.getElementById('kpi-agents-val').textContent = activeCount;
    document.getElementById('agent-count').textContent = `${activeCount} Agents`;
    document.getElementById('agents-online').textContent = `${activeCount} Online`;

    document.getElementById('kpi-tasks-val').textContent = state.totalTasksProcessed;
    document.getElementById('kpi-auctions-val').textContent = state.auctionsWon;

    // Avg latency
    if (state.latencies.length > 0) {
        const avg = Math.round(state.latencies.reduce((a, b) => a + b, 0) / state.latencies.length);
        document.getElementById('kpi-latency-val').innerHTML = `${avg}<span class="kpi-unit">ms</span>`;
    }

    // Avg reputation
    const reps = [...state.agents.values()].filter(a => a.joined).map(a => a.reputation);
    if (reps.length > 0) {
        const avgRep = Math.round(reps.reduce((a, b) => a + b, 0) / reps.length);
        document.getElementById('kpi-rep-val').textContent = avgRep;
    }

    // Pipeline counts
    const auctioning = [...state.tasks.values()].filter(t => t.status === 'auctioning').length;
    const executing = [...state.tasks.values()].filter(t => t.status === 'executing').length;
    const done = [...state.tasks.values()].filter(t => t.status === 'completed').length;

    document.getElementById('stage-auction').textContent = auctioning;
    document.getElementById('stage-executing').textContent = executing;
    document.getElementById('stage-done').textContent = done;
    document.getElementById('pipeline-count').textContent = `${auctioning + executing} Active`;
}

// ═══════════════════════════════════════════
// Task Pipeline Rendering
// ═══════════════════════════════════════════
function renderPipeline() {
    const auctionEl = document.getElementById('auction-items');
    const execEl = document.getElementById('executing-items');
    const doneEl = document.getElementById('done-items');

    auctionEl.innerHTML = '';
    execEl.innerHTML = '';
    doneEl.innerHTML = '';

    state.tasks.forEach((task) => {
        const item = document.createElement('div');
        item.className = 'task-item';
        item.innerHTML = `
            <div class="task-item-id">${task.id}</div>
            <div class="task-item-desc">${task.desc}</div>
            ${task.progress > 0 ? `
                <div class="task-progress">
                    <div class="task-progress-fill" style="width: ${task.progress}%"></div>
                </div>
            ` : ''}
        `;

        if (task.status === 'auctioning') auctionEl.appendChild(item);
        else if (task.status === 'executing') execEl.appendChild(item);
        else if (task.status === 'completed') doneEl.appendChild(item);
    });
}

// ═══════════════════════════════════════════
// Simulation Engine
// ═══════════════════════════════════════════
let simStep = 0;

function simulateStep() {
    simStep++;

    // Phase 1: Agents join one by one (first 16 seconds)
    if (simStep <= AGENTS.length * 2) {
        const agentIdx = Math.floor((simStep - 1) / 2);
        if (agentIdx < AGENTS.length) {
            const agent = state.agents.get(AGENTS[agentIdx].id);
            if (!agent.joined) {
                agent.joined = true;
                agent.status = 'active';
                agent.lastSeen = Date.now();
                addLog('👋', `<strong>${agent.name}</strong> joined the swarm [HELLO handshake]`, 'hello');
                state.latencies.push(Math.floor(Math.random() * 60 + 30));
                renderAgents();
                updateKPIs();
                resizeTopoCanvas();
            }
        }
        return;
    }

    // Phase 2: Normal operation
    const actions = [
        'heartbeat', 'heartbeat', 'heartbeat',
        'task', 'task',
        'bid', 'bid',
        'complete',
        'role-change',
        'stale',
        'recover',
    ];

    const action = actions[Math.floor(Math.random() * actions.length)];
    const activeAgents = [...state.agents.values()].filter(a => a.status === 'active' || a.status === 'busy');

    if (activeAgents.length === 0) return;
    const randomAgent = activeAgents[Math.floor(Math.random() * activeAgents.length)];

    switch (action) {
        case 'heartbeat': {
            randomAgent.lastSeen = Date.now();
            state.latencies.push(Math.floor(Math.random() * 50 + 20));
            if (state.latencies.length > 50) state.latencies.shift();
            // Don't log heartbeats to avoid spam
            break;
        }

        case 'task': {
            const taskDef = TASK_POOL[Math.floor(Math.random() * TASK_POOL.length)];
            state.taskCounter++;
            const taskId = `task-${String(state.taskCounter).padStart(3, '0')}`;
            state.tasks.set(taskId, {
                id: taskId,
                desc: taskDef.desc,
                caps: taskDef.caps,
                complexity: taskDef.complexity,
                status: 'auctioning',
                progress: 0,
                assignee: null,
                reward: taskDef.complexity * 20,
            });

            addLog('📢', `<strong>${randomAgent.name}</strong> broadcast task: "${taskDef.desc}"`, 'task');
            renderPipeline();

            // Auto-resolve auction after delay
            setTimeout(() => {
                const task = state.tasks.get(taskId);
                if (task && task.status === 'auctioning') {
                    // Pick a winner from capable agents
                    const capable = activeAgents.filter(a =>
                        task.caps.some(c => a.caps.includes(c)) && a.activeTasks < 3
                    );
                    if (capable.length > 0) {
                        const best = capable.reduce((a, b) => a.reputation > b.reputation ? a : b);
                        task.status = 'executing';
                        task.assignee = best.id;
                        best.activeTasks++;
                        best.status = 'busy';
                        state.auctionsWon++;

                        addLog('🏆', `<strong>${best.name}</strong> won auction for "${task.desc}" (${capable.length} bids)`, 'win');
                        renderAgents();
                        renderPipeline();
                        updateKPIs();

                        // Simulate execution progress
                        let progress = 0;
                        const progressInterval = setInterval(() => {
                            progress += Math.floor(Math.random() * 25 + 10);
                            if (progress >= 100) {
                                progress = 100;
                                clearInterval(progressInterval);
                                task.status = 'completed';
                                task.progress = 100;
                                best.activeTasks = Math.max(0, best.activeTasks - 1);
                                best.reputation += 10;
                                if (best.activeTasks === 0) best.status = 'active';
                                state.totalTasksProcessed++;

                                addLog('✅', `<strong>${best.name}</strong> completed "${task.desc}" (+10 rep)`, 'complete');
                                renderAgents();
                                renderPipeline();
                                updateKPIs();
                            } else {
                                task.progress = progress;
                                renderPipeline();
                            }
                        }, 2000 + Math.random() * 2000);
                    }
                }
            }, 3000 + Math.random() * 2000);
            break;
        }

        case 'bid': {
            const auctioning = [...state.tasks.values()].filter(t => t.status === 'auctioning');
            if (auctioning.length > 0) {
                const task = auctioning[Math.floor(Math.random() * auctioning.length)];
                const price = task.complexity * 10 + Math.floor(Math.random() * 50);
                addLog('💰', `<strong>${randomAgent.name}</strong> bid $${price} on "${task.desc}"`, 'bid');
            }
            break;
        }

        case 'complete': {
            // Handled in task flow above
            break;
        }

        case 'role-change': {
            const roles = ['worker', 'coordinator', 'validator', 'observer'];
            const newRole = roles[Math.floor(Math.random() * roles.length)];
            if (newRole !== randomAgent.role) {
                const oldRole = randomAgent.role;
                randomAgent.role = newRole;
                addLog('🔄', `<strong>${randomAgent.name}</strong> role: ${oldRole} → ${newRole}`, 'hello');
                renderAgents();
            }
            break;
        }

        case 'stale': {
            if (activeAgents.length > 3) {
                const victim = activeAgents[Math.floor(Math.random() * activeAgents.length)];
                if (victim.status === 'active') {
                    victim.status = 'stale';
                    victim.reputation -= 5;
                    addLog('⚠️', `<strong>${victim.name}</strong> marked STALE (no heartbeat >10s)`, 'stale');

                    // Reassign victim's tasks
                    state.tasks.forEach((task) => {
                        if (task.assignee === victim.id && task.status === 'executing') {
                            task.status = 'auctioning';
                            task.assignee = null;
                            task.progress = 0;
                            addLog('🔄', `Task "${task.desc}" re-auctioned (agent went stale)`, 'stale');
                        }
                    });

                    renderAgents();
                    renderPipeline();
                    updateKPIs();
                }
            }
            break;
        }

        case 'recover': {
            const staleAgents = [...state.agents.values()].filter(a => a.status === 'stale');
            if (staleAgents.length > 0) {
                const recovered = staleAgents[0];
                recovered.status = 'active';
                recovered.lastSeen = Date.now();
                addLog('✅', `<strong>${recovered.name}</strong> RECOVERED and re-joined the swarm`, 'recover');
                renderAgents();
                updateKPIs();
            }
            break;
        }
    }

    updateKPIs();
}

// ═══════════════════════════════════════════
// Start Simulation
// ═══════════════════════════════════════════
addLog('🚀', 'SwarmMind Dashboard initialized — waiting for agents...', 'hello');
addLog('🔗', 'Vertex BFT consensus engine ready', 'hello');

// Agents join every 2 seconds
setInterval(simulateStep, 2000);

// Initial render
renderAgents();
updateKPIs();
renderPipeline();
