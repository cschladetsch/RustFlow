# RustFlow Architecture Diagrams

## Project Architecture Overview

```mermaid
graph TB
    subgraph "RustFlow Core System"
        K[Kernel]
        R[Runtime]
        F[Factory]
        
        K --> R
        F --> K
    end
    
    subgraph "Flow Components"
        N[Node]
        G[Group] 
        S[Sequence]
        B[Barrier]
        FU[Future]
        T[Timer]
        CH[Channel]
        CO[Coroutine]
    end
    
    subgraph "Base Traits"
        TR[Transient]
        ST[Steppable]
        GE[Generator]
    end
    
    F --> N
    F --> G
    F --> S
    F --> B
    F --> FU
    F --> T
    F --> CH
    F --> CO
    
    N --> TR
    G --> TR
    S --> ST
    B --> ST
    FU --> GE
    T --> GE
    CH --> GE
    CO --> GE
    
    ST --> TR
    GE --> ST
```

## Class Hierarchy and Trait Relationships

```mermaid
classDiagram
    class Transient {
        <<trait>>
        +id() TransientId
        +name() Option~str~
        +set_name(name: String)
        +is_active() bool
        +is_completed() bool
        +complete()
        +step() Result~()~
        +resume()
        +suspend()
    }
    
    class Steppable {
        <<trait>>
    }
    
    class Generator {
        <<trait>>
        +state() GeneratorState
        +step_number() u64
        +value() Option~Output~
        +pre()
        +post()
        +is_running() bool
    }
    
    class Node {
        +new() Self
        +add(transient)
        +remove(id)
    }
    
    class Group {
        +new() Self
        +add(transient)
        +remove(id)
    }
    
    class Sequence {
        +new() Self
        +add_step(step)
        +current_step() Option~usize~
    }
    
    class Barrier {
        +new() Self
        +add_dependency(dep)
        +wait()
    }
    
    class Future {
        +new() Self
        +set_value(value)
        +get_value() Option~T~
    }
    
    class Timer {
        +new(duration) Self
        +remaining() Duration
    }
    
    class Channel {
        +new() Self
        +send(message)
        +receive() Option~T~
    }
    
    Transient <|-- Steppable
    Steppable <|-- Generator
    Transient <|.. Node
    Transient <|.. Group
    Steppable <|.. Sequence
    Steppable <|.. Barrier
    Generator <|.. Future
    Generator <|.. Timer
    Generator <|.. Channel
```

## Module Structure

```mermaid
graph LR
    subgraph "src/"
        LIB[lib.rs]
        
        subgraph "Core Modules"
            TRAITS[traits.rs]
            TYPES[types.rs]
            ERROR[error.rs]
            KERNEL[kernel.rs]
            RUNTIME[runtime.rs]
            FACTORY[factory.rs]
        end
        
        subgraph "flow/"
            BARRIER[barrier.rs]
            CHANNEL[channel.rs]
            COROUTINE[coroutine.rs]
            FUTURE[future.rs]
            GROUP[group.rs]
            NODE[node.rs]
            SEQUENCE[sequence.rs]
            TIMER[timer.rs]
        end
        
        subgraph "Support Modules"
            THREADING[threading.rs]
            LOGGER[logger.rs]
        end
    end
    
    LIB --> TRAITS
    LIB --> TYPES
    LIB --> ERROR
    LIB --> KERNEL
    LIB --> RUNTIME
    LIB --> FACTORY
    LIB --> THREADING
    LIB --> LOGGER
    
    FACTORY --> BARRIER
    FACTORY --> CHANNEL
    FACTORY --> COROUTINE
    FACTORY --> FUTURE
    FACTORY --> GROUP
    FACTORY --> NODE
    FACTORY --> SEQUENCE
    FACTORY --> TIMER
```

## Runtime Execution Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant R as Runtime
    participant K as Kernel
    participant F as Factory
    participant FC as Flow Components
    
    App->>R: Runtime::new()
    R->>K: Create Kernel
    K->>F: Create Factory
    
    App->>K: kernel.read().await
    App->>F: factory.timer/sequence/etc
    F->>FC: Create components
    App->>K: root.add(component)
    
    App->>R: run_for(duration, frame_time)
    
    loop Every Frame
        R->>K: step()
        K->>FC: step() on each component
        FC-->>K: Result
        K-->>R: Frame complete
        R->>R: Sleep until next frame
    end
    
    R-->>App: Runtime complete
```

## Flow Component Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Created
    Created --> Active: resume()
    Active --> Running: step()
    Running --> Running: step() continues
    Running --> Suspended: suspend()
    Running --> Completed: complete()
    Suspended --> Active: resume()
    Completed --> [*]
    
    note right of Running
        Components execute their
        logic during step()
    end note
    
    note right of Completed
        Component is finished
        and can be removed
    end note
```

## Threading vs Async Architecture

```mermaid
graph TB
    subgraph "RustFlow Execution Models"
        subgraph "Async Model"
            AR[Async Runtime]
            TK[Tokio]
            AF[Async Futures]
            AW[Async/Await]
            
            AR --> TK
            TK --> AF
            AF --> AW
        end
        
        subgraph "Threading Model"
            TH[Threading Module]
            CB[Crossbeam]
            TP[Thread Pools]
            SH[Shared State]
            
            TH --> CB
            CB --> TP
            TP --> SH
        end
        
        subgraph "Flow Components"
            FC[Flow Components]
            FC --> AR
            FC --> TH
        end
    end
    
    subgraph "Use Cases"
        IO[I/O Bound Tasks] --> AR
        CPU[CPU Bound Tasks] --> TH
        MX[Mixed Workloads] --> FC
    end
```

## Component Interaction Patterns

```mermaid
graph TB
    subgraph "Coordination Patterns"
        subgraph "Sequential"
            S1[Step 1] --> S2[Step 2]
            S2 --> S3[Step 3]
            S3 --> S4[Complete]
        end
        
        subgraph "Parallel"
            P1[Task 1]
            P2[Task 2] 
            P3[Task 3]
            P1 --> B1[Barrier]
            P2 --> B1
            P3 --> B1
            B1 --> PC[Continue]
        end
        
        subgraph "Producer-Consumer"
            PROD[Producer] --> CH1[Channel]
            CH1 --> CONS[Consumer]
        end
        
        subgraph "Future-Based"
            REQ[Request] --> FUT[Future]
            FUT --> RES[Result Available]
        end
    end
```