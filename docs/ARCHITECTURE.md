# Qwen Studio Architecture

> **Version**: 2.2.0 | **Last Updated**: 2026-05-20  
> **Stack**: Tauri v2 + WebKitGTK + Rust

## Migration from Electron (v2.2.0)

**v2.2.0** migrates from Electron to Tauri v2 for:
- **95% smaller binary**: ~6MB (Tauri) vs ~150MB (Electron)
- **Better performance**: Rust backend, native WebKitGTK WebView
- **Native Linux integration**: System tray, GTK menu, deep linking
- **Improved security**: Sandboxed WebView, no Node.js in renderer

**Key changes:**
- Main process: Node.js → Rust
- WebView: Chromium → WebKitGTK
- IPC: Electron IPC → Tauri commands (`invoke`)
- Bundling: electron-builder → Tauri bundler (RPM/DEB/AppImage)

## System Overview

```mermaid
flowchart TB
    subgraph User["User Interface Layer"]
        WebView["chat.qwen.ai WebView<br/>(WebKitGTK)"]
        Settings["Settings UI<br/>(Injected JS)"]
        SystemTray["System Tray"]
    end

    subgraph Tauri["Tauri Application"]
        subgraph Rust["Rust Backend"]
            Lib["lib.rs<br/>(Bootstrap & Commands)"]
            Window["window.rs<br/>(Window Management)"]
            IPC["ipc-handlers<br/>(Command Bridge)"]
            MCP["mcp.rs<br/>(MCP Server Management)"]
            Tray["tray.rs<br/>(System Tray)"]
            Menu["menu.rs<br/>(GTK HeaderBar)"]
        end

        subgraph Web["Web Injection"]
            Bridge["electron-bridge.js<br/>(window.__TAURI__)"]
            MCPBridge["mcp-bridge.mjs<br/>(MCP Proxy)"]
        end
    end

    subgraph Servers["MCP Servers (stdio)"]
        QwenCore["qwen-core<br/>(28 tools)"]
        Fetch["fetch MCP"]
        FS["filesystem MCP"]
    end

    subgraph External["External Systems"]
        QwenAI["chat.qwen.ai<br/>(Alibaba Cloud)"]
        Settings["~/.config/qwen-studio/<br/>settings.json"]
        IndexedDB["IndexedDB<br/>(Conversation State)"]
        FileSystem["Local Filesystem"]
    end

    WebView -->|JS Bridge| Bridge
    Bridge -->|invoke| IPC
    IPC -->|Rust commands| Lib
    Lib -->|creates| Window
    Lib -->|owns| MCP
    MCP -->|spawns| QwenCore
    MCP -->|spawns| Fetch
    MCP -->|spawns| FS
    
    Lib -->|read/write| Settings
    Lib -->|inject| Bridge
    Window -->|loads URL| QwenAI
    QwenAI -->|stores| IndexedDB
    QwenAI -->|console messages| Window
    
    QwenCore -->|read/write| FileSystem
    Fetch -->|HTTP requests| External
    FS -->|file operations| FileSystem
    
    Lib -->|manages| SystemTray
    Lib -->|builds| Menu
```

## Data Flow: MCP Tool Execution

```mermaid
sequenceDiagram
    participant UI as chat.qwen.ai<br/>(WebView)
    participant Preload as Preload Script<br/>(contextBridge)
    participant IPC as IPC Handlers
    participant Proxy as MCP Proxy
    participant Client as MCP Server Client
    participant Server as qwen-core<br/>(Bun Process)

    UI->>Preload: window.electronAPI.mcp_client_tool_call({server, tool, args})
    Preload->>IPC: ipcRenderer.invoke("mcp_client_tool_call", params)
    IPC->>Proxy: callTool(params)
    Proxy->>Client: callTool()
    Client->>Server: stdio write {jsonrpc, method, params}
    
    Note over Server: Executes tool<br/>(file read, git, etc.)
    
    Server-->>Client: stdio read {jsonrpc, result}
    Client-->>Proxy: Promise<result>
    Proxy-->>IPC: result
    IPC-->>Preload: ipcRenderer result
    Preload-->>UI: Promise resolves
    
    Note over UI: Renders tool output<br/>in chat interface
```

## Component Responsibilities

```mermaid
mindmap
  root((Qwen Studio))
    Rust Backend
      lib.rs
        ::icon(fa fa-flag)
        Bootstrap
        WebView Setup
        Update Commands
        Zoom Controls
      window.rs
        ::icon(fa fa-window)
        WebviewWindow
        Deep Link Handling
        Theme Switching
      mcp.rs
        ::icon(fa fa-cog)
        MCP Server State
        Config Sync
      tray.rs
        ::icon(fa fa-tray)
        System Tray
        Menu Items
      menu.rs
        ::icon(fa fa-bars)
        GTK HeaderBar
        Linux Menu
    Web Injection
      electron-bridge.js
        ::icon(fa fa-bridge)
        window.__TAURI__ shim
        Event Forwarding
      mcp-bridge.mjs
        ::icon(fa fa-exchange)
        MCP Proxy Server
        stdio Transport
    MCP Servers
      qwen-core
        ::icon(fa fa-robot)
        28 Tools
        Skills System
      fetch
        ::icon(fa fa-globe)
        Web Access
      filesystem
        ::icon(fa fa-file)
        File Operations
    External
      Settings
        ::icon(fa fa-file)
        ~/.config/qwen-studio/
        MCP Config
      IndexedDB
        ::icon(fa fa-database)
        Conversation State
        chat.qwen.ai
```

## Process Architecture

```mermaid
flowchart LR
    subgraph P1["Rust Backend (Main Process)"]
        M1[WebviewWindow]
        M2[MCP Manager]
        M3[Tauri Commands]
    end

    subgraph P2["Renderer (WebKitGTK)"]
        R1[chat.qwen.ai WebView]
        R2[electron-bridge.js]
        R3[Settings Tab Injection]
    end

    subgraph P3["MCP Servers (stdio)"]
        S1[qwen-core (Bun)]
        S2[fetch (Node.js)]
        S3[filesystem (Node.js)]
    end

    P1 <-->|Tauri Commands| P2
    P1 <-->|stdio JSON-RPC| P3
    
    style P1 fill:#e1f5ff
    style P2 fill:#fff4e1
    style P3 fill:#e8f5e9
```

**Changes from Electron:**
- Single Rust process replaces Node.js main process
- WebKitGTK replaces Chromium renderer
- stdio transport unchanged for MCP servers
- IPC: `ipcRenderer.invoke()` → `window.__TAURI__.core.invoke()`

## MCP Server Lifecycle

```mermaid
stateDiagram-v2
    [*] --> ConfigLoaded: app start
    ConfigLoaded --> Connecting: mcp_client_connect
    Connecting --> Connected: stdio spawn success
    Connecting --> Error: spawn fails
    Connected --> Executing: callTool
    Executing --> Connected: result returned
    Connected --> Disconnecting: mcp_client_close / app quit
    Disconnecting --> [*]: process killed
    Error --> [*]: error logged

    note right of Connected
        Client cached in
        McpProxy.clients Map
    end note

    note left of Executing
        Tool runs in isolated
        Bun/Python process
    end note
```

## File Structure Map

```mermaid
graph TD
    A[qwen-studio/] --> B[src/]
    A --> C[qwen-core/]
    A --> D[docs/]
    A --> E[Config Files]

    B --> B1[lib.rs - Bootstrap]
    B --> B2[main.rs - Entry]
    B --> B3[window.rs - Window Mgmt]
    B --> B4[mcp.rs - MCP State]
    B --> B5[tray.rs - System Tray]
    B --> B6[menu.rs - GTK Menu]
    B --> B7[dialogs.rs - Native Dialogs]
    B --> B8[events.rs - Event Forwarding]
    B --> B9[settings.rs - Settings Storage]

    C --> C1[src/index.ts - MCP Server]
    C --> C2[skills/ - Agent Skills]
    C --> C3[package.json]

    D --> D1[ARCHITECTURE.md]
    D --> D2[AGENT_BRAIN.md]
    D --> D3[SECURITY_AUDIT.md]

    E --> E1[tauri.conf.json]
    E --> E2[Cargo.toml]
    E --> E3[electron-bridge.js]
    E --> E4[mcp-bridge.mjs]
```

## Tauri Command Map

| Command | Handler | Purpose |
|---------|---------|---------|
| `get_update_info` | `lib.rs:804` | Check for updates |
| `install_update_with_progress` | `lib.rs:724` | Download + install |
| `restart_app` | `lib.rs:790` | Restart after update |
| `mcp_client_update_config` | `mcp.rs` | Update MCP config |
| `get_setting` | `settings.rs` | Read settings |
| `set_setting` | `settings.rs` | Write settings |
| `switch_theme` | `window.rs` | Toggle dark/light |
| `switch_ln` | `window.rs` | Change language |
| `webview_loaded` | `events.rs` | WebView ready event |

**IPC Migration:**
- Electron: `ipcRenderer.invoke('channel', args)`
- Tauri: `window.__TAURI__.core.invoke('command', args)`

## Key Design Decisions

### 1. Tauri v2 Migration (v2.2.0)
**Decision:** Migrate from Electron to Tauri v2 (Rust + WebKitGTK)

**Why:**
- 95% smaller binary (~6MB vs ~150MB)
- Better performance with Rust backend
- Native Linux system tray and menu
- Improved security (sandboxed WebView)
- Lower memory footprint

**Trade-offs:**
- WebKitGTK instead of Chromium (minor rendering differences)
- Rust learning curve for future contributors
- AppImage bundling challenges (linuxdeploy issues)

### 2. WebView Wrapper Pattern
**Decision:** Wrap chat.qwen.ai instead of building native chat UI

**Why:**
- Leverages Alibaba's continuous web app improvements
- No need to implement chat rendering, message history, account management
- Focus on desktop integration (MCP, filesystem, system tray)

**Trade-off:** Cannot modify chat UI behavior; dependent on web app stability

### 3. MCP Proxy Architecture
**Decision:** Single proxy managing multiple server connections via stdio

**Why:**
- Unified API for renderer
- Client caching reduces spawn overhead
- Lazy connection model (connect on first tool call)

**Implementation:** `mcp-bridge.mjs` - Node.js proxy server

### 4. stdio Transport for MCP
**Decision:** Use stdio JSON-RPC instead of HTTP/SSE for local servers

**Why:**
- No network overhead
- Automatic cleanup on process exit
- Simpler security model (no open ports)

**Protocol:** JSON-RPC 2.0 over stdin/stdout

### 5. qwen-core Embedding
**Decision:** Bundle qwen-core as external npm package, not inside app archive

**Why:**
- Easy updates via npm
- Version independent from app version
- Standard Node.js module resolution

**Location:** `~/.config/qwen-studio/node_modules/qwen-core`

## Error Recovery: parent_id Flow

```mermaid
flowchart TD
    A[User sends message] --> B{Server validates<br/>parent_id}
    B -->|Valid| C[Message accepted]
    B -->|Invalid/Expired| D[Error: parent_id is not exist]
    
    D --> E[WebView console.error]
    E --> F[window-manager.ts<br/>console-message listener]
    
    F --> G{Detects pattern?<br/>parent_id + is not exist}
    G -->|No| H[Error shown to user]
    G -->|Yes| I[Log: ⚠️ parent_id error]
    
    I --> J[Clear IndexedDB + localStorage]
    J --> K[Show toast:<br/>Session refreshed]
    K --> L[Wait 500ms]
    L --> M[mainWindow.reload]
    M --> N[Fresh session<br/>new parent_id]
    
    style F fill:#ffeb3b
    style J fill:#a5d6a7
    style M fill:#90caf9
```

## Build Pipeline

```mermaid
flowchart LR
    A[npm install] --> B[postinstall:<br/>download MCP runtimes]
    
    C[npm run tauri:build] --> D[cargo build --release]
    D --> E[Tauri Bundler]
    E --> F[DEB Package]
    E --> G[RPM Package]
    E --> H[AppImage (optional)]
    
    F --> I[target/release/bundle/deb/]
    G --> J[target/release/bundle/rpm/]
    H --> K[target/release/bundle/appimage/]
    
    style B fill:#ffe0b2
    style E fill:#c8e6c9
    style I fill:#bbdefb
```

**Build Commands:**
```bash
# Development (hot reload)
npm run tauri:dev

# Build all formats
npm run tauri:build

# Individual formats
npm run tauri:build:deb    # Debian/Ubuntu
npm run tauri:build:rpm    # Fedora/RHEL
```

**RPM Fix:** Uses `"compression": { "type": "none" }` to avoid `rpm-rs` gzip stall on large binaries.

## Configuration Storage

| Config | Location | Format | Managed By |
|--------|----------|--------|------------|
| MCP Servers | `~/.config/qwen-studio/settings.json` | `{ mcpServers: {...} }` | Tauri commands |
| App Theme | Web app account settings | Server-side | chat.qwen.ai |
| Language | `~/.config/qwen-studio/settings.json` | `{ app_language: "en" }` | Tauri commands |
| Conversation State | IndexedDB (LevelDB) | Binary (Leveldb) | chat.qwen.ai |
| Skills | `~/.config/qwen-studio/skills/` | Markdown files | qwen-core |

## Security Boundaries

```mermaid
flowchart TB
    subgraph Trusted["Trusted Zone (Main Process)"]
        M1[Node.js APIs]
        M2[Filesystem Access]
        M3[MCP Server Spawn]
        M4[System Tray]
    end

    subgraph Boundary["Security Boundary"]
        CB[contextBridge]
        IPC[IPC Handlers]
    end

    subgraph Untrusted["Untrusted Zone (Renderer)"]
        R1[chat.qwen.ai]
        R2[Third-party scripts]
        R3[User-generated content]
    end

    Untrusted -->|Cannot access directly| Boundary
    Boundary -->|Whitelisted APIs only| Trusted
    
    style Trusted fill:#c8e6c9
    style Boundary fill:#ffecb3
    style Untrusted fill:#ffcdd2
```

---

**Generated:** 2026-05-20  
**Version:** qwen-studio v2.2.0 (Tauri v2)
