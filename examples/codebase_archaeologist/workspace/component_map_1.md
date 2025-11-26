# Component Map

```mermaid
graph TD
    Main[Main Entry] --> Config[Configuration]
    Main --> Agent[Agent Loop]
    Agent --> Client[Unified Client]
    Client --> Provider[Provider Logic]
    Provider --> Models[Shared Models]
    
    subgraph Core
        Agent
        Config
    end
    
    subgraph Connectivity
        Client
        Provider
        Models
    end
```

## Components
- **Main Entry**: Application bootstrap.
- **Agent Loop**: Core DMN autonomous loop.
- **Configuration**: Settings and environment management.
- **Unified Client**: HTTP transport to LLMs.
- **Provider Logic**: Model detection and API normalization.
- **Shared Models**: Data structures for API communication.
