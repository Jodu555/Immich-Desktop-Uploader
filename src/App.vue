<template>
  <div class="container">
    <header>
      <h1>Immich Auto Uploader</h1>
      <div class="connection-status" :class="connectionStatus">
        {{ connectionStatus === 'connected' ? '● Connected' : '○ Disconnected' }}
      </div>
    </header>

    <section class="config-section">
      <h2>Immich Configuration</h2>
      <div class="form-group">
        <label>Server URL:</label>
        <input v-model="config.serverUrl" placeholder="https://immich.example.com" />
      </div>
      <div class="form-group">
        <label>API Key:</label>
        <input v-model="config.apiKey" type="password" placeholder="Your API key" />
      </div>
      <button @click="testConnection" :disabled="isTestingConnection">
        {{ isTestingConnection ? 'Testing...' : 'Test Connection' }}
      </button>
    </section>

    <section class="upload-paths-section">
      <h2>Upload Paths</h2>
      <div v-for="(path, idx) in uploadPaths" :key="idx" class="path-item">
        <div class="path-header">
          <h3>Path {{ idx + 1 }}</h3>
          <button @click="removePath(idx)" class="btn-danger">Remove</button>
        </div>
        <div class="form-group">
          <label>Folder Path:</label>
          <div class="path-input-group">
            <input v-model="path.directory" placeholder="/path/to/images" readonly />
            <button @click="selectDirectory(idx)">Browse</button>
          </div>
        </div>
        <div class="form-group">
          <label>CRON Expressions (one per line):</label>
          <textarea v-model="path.cronExpressions"
            placeholder="0 0 * * * (daily at midnight)&#10;0 12 * * 0 (Sundays at noon)" rows="3"></textarea>
          <small>Examples: "0 0 * * *" (daily), "0 */6 * * *" (every 6 hours)</small>
        </div>
        <div class="form-group">
          <label>
            <input type="checkbox" v-model="path.recursive" />
            Include subdirectories
          </label>
        </div>
        <div class="next-run" v-if="path.nextRun">
          Next upload: {{ new Date(path.nextRun).toLocaleString() }}
        </div>
      </div>
      <button @click="addPath" class="btn-add">+ Add Path</button>
    </section>

    <section class="actions-section">
      <button @click="saveConfig" class="btn-primary">Save Configuration</button>
      <button @click="startScheduler" :disabled="isSchedulerRunning" class="btn-success">
        Start Scheduler
      </button>
      <button @click="stopScheduler" :disabled="!isSchedulerRunning" class="btn-warning">
        Stop Scheduler
      </button>
    </section>

    <section class="logs-section">
      <h2>Upload Logs</h2>
      <div class="log-container">
        <div v-for="(log, idx) in logs" :key="idx" :class="['log-entry', log.level]">
          <span class="log-time">{{ new Date(log.timestamp).toLocaleTimeString() }}</span>
          <span class="log-message">{{ log.message }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';

interface Config {
  serverUrl: string;
  apiKey: string;
}

interface UploadPath {
  directory: string;
  cronExpressions: string;
  recursive: boolean;
  nextRun?: number;
}

interface LogEntry {
  timestamp: number;
  level: 'info' | 'error' | 'success';
  message: string;
}

const config = ref<Config>({
  serverUrl: '',
  apiKey: ''
});

const uploadPaths = ref<UploadPath[]>([]);
const logs = ref<LogEntry[]>([]);
const connectionStatus = ref<'connected' | 'disconnected'>('disconnected');
const isTestingConnection = ref(false);
const isSchedulerRunning = ref(false);

const addLog = (message: string, level: 'info' | 'error' | 'success' = 'info') => {
  logs.value.unshift({
    timestamp: Date.now(),
    level,
    message
  });
  if (logs.value.length > 100) logs.value.pop();
};

const testConnection = async () => {
  isTestingConnection.value = true;
  try {
    const result = await invoke<boolean>('test_immich_connection', {
      serverUrl: config.value.serverUrl,
      apiKey: config.value.apiKey
    });

    if (result) {
      connectionStatus.value = 'connected';
      addLog('Successfully connected to Immich server', 'success');
    } else {
      connectionStatus.value = 'disconnected';
      addLog('Failed to connect to Immich server', 'error');
    }
  } catch (e) {
    connectionStatus.value = 'disconnected';
    addLog(`Connection error: ${e}`, 'error');
  } finally {
    isTestingConnection.value = false;
  }
};

const selectDirectory = async (idx: number) => {
  const selected = await open({
    directory: true,
    multiple: false
  });

  if (selected && typeof selected === 'string') {
    uploadPaths.value[idx].directory = selected;
  }
};

const addPath = () => {
  uploadPaths.value.push({
    directory: '',
    cronExpressions: '',
    recursive: true
  });
};

const removePath = (idx: number) => {
  uploadPaths.value.splice(idx, 1);
};

const saveConfig = async () => {
  try {
    await invoke('save_config', {
      config: {
        server_url: config.value.serverUrl,
        api_key: config.value.apiKey,
        paths: uploadPaths.value.map(p => ({
          directory: p.directory,
          cronExpressions: p.cronExpressions.split('\n').filter(c => c.trim()),
          recursive: p.recursive
        }))
      }
    });
    addLog('Configuration saved successfully', 'success');
  } catch (e) {
    addLog(`Failed to save configuration: ${e}`, 'error');
  }
};

const startScheduler = async () => {
  try {
    await invoke('start_scheduler');
    isSchedulerRunning.value = true;
    addLog('Scheduler started', 'success');
  } catch (e) {
    addLog(`Failed to start scheduler: ${e}`, 'error');
  }
};

const stopScheduler = async () => {
  try {
    await invoke('stop_scheduler');
    isSchedulerRunning.value = false;
    addLog('Scheduler stopped', 'info');
  } catch (e) {
    addLog(`Failed to stop scheduler: ${e}`, 'error');
  }
};

const loadConfig = async () => {
  try {
    const savedConfig = await invoke<any>('load_config');
    if (savedConfig) {
      config.value.serverUrl = savedConfig.server_url || '';
      config.value.apiKey = savedConfig.api_key || '';
      uploadPaths.value = (savedConfig.paths || []).map((p: any) => ({
        directory: p.directory,
        cronExpressions: p.cronExpressions.join('\n'),
        recursive: p.recursive ?? true
      }));
    }
  } catch (e) {
    addLog('No saved configuration found', 'info');
  }
};

const loadSchedulerStatus = async () => {
  const result = await invoke<any>('status_scheduler');
  isSchedulerRunning.value = result;
};

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await loadConfig();
  await loadSchedulerStatus();


  unlisten = await listen('upload-event', (event: any) => {
    const { type, message } = event.payload;
    addLog(message, type);
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});
</script>

<style scoped>
.container {
  padding: 20px;
  max-width: 1000px;
  margin: 0 auto;
  font-family: system-ui, -apple-system, sans-serif;
}

header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
  padding-bottom: 15px;
  border-bottom: 2px solid #e0e0e0;
}

h1 {
  margin: 0;
  color: #333;
}

.connection-status {
  font-weight: 600;
  padding: 8px 16px;
  border-radius: 20px;
}

.connection-status.connected {
  color: #22c55e;
  background: #f0fdf4;
}

.connection-status.disconnected {
  color: #ef4444;
  background: #fef2f2;
}

section {
  margin-bottom: 30px;
  padding: 20px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

h2 {
  margin-top: 0;
  color: #555;
  font-size: 1.3em;
}

.form-group {
  margin-bottom: 15px;
}

label {
  display: block;
  margin-bottom: 5px;
  font-weight: 500;
  color: #666;
}

input[type="text"],
input[type="password"],
textarea {
  width: 100%;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  box-sizing: border-box;
}

textarea {
  font-family: monospace;
  resize: vertical;
}

.path-input-group {
  display: flex;
  gap: 10px;
}

.path-input-group input {
  flex: 1;
}

button {
  padding: 10px 20px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: opacity 0.2s;
}

button:hover:not(:disabled) {
  opacity: 0.8;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #3b82f6;
  color: white;
}

.btn-success {
  background: #22c55e;
  color: white;
}

.btn-warning {
  background: #f59e0b;
  color: white;
}

.btn-danger {
  background: #ef4444;
  color: white;
  padding: 6px 12px;
  font-size: 12px;
}

.btn-add {
  background: #8b5cf6;
  color: white;
  width: 100%;
}

.path-item {
  border: 1px solid #e0e0e0;
  padding: 15px;
  margin-bottom: 15px;
  border-radius: 6px;
  background: #fafafa;
}

.path-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.path-header h3 {
  margin: 0;
  color: #666;
}

.next-run {
  margin-top: 10px;
  padding: 8px;
  background: #eff6ff;
  border-radius: 4px;
  color: #1e40af;
  font-size: 13px;
}

.actions-section {
  display: flex;
  gap: 10px;
}

.actions-section button {
  flex: 1;
}

.log-container {
  max-height: 300px;
  overflow-y: auto;
  background: #1e1e1e;
  padding: 15px;
  border-radius: 4px;
}

.log-entry {
  padding: 6px 0;
  font-family: monospace;
  font-size: 13px;
  border-bottom: 1px solid #333;
}

.log-entry:last-child {
  border-bottom: none;
}

.log-entry.info {
  color: #93c5fd;
}

.log-entry.error {
  color: #fca5a5;
}

.log-entry.success {
  color: #86efac;
}

.log-time {
  color: #9ca3af;
  margin-right: 10px;
}

small {
  display: block;
  margin-top: 5px;
  color: #999;
  font-size: 12px;
}
</style>