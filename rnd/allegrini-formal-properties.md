                                              Formalizing the Safety, Security, and Functional
                                                    Properties of Agentic AI Systems
                                                              Edoardo Allegrini                          Ananth Shreekumar                      Z. Berkay Celik
                                                        Sapienza University of Rome                        Purdue University                   Purdue University
                                                          allegrini@di.uniroma1.it                       ashreeku@purdue.edu                   zcelik@purdue.edu



                                            Abstract—Agentic AI systems, which leverage multiple au-                The operation of such systems relies on standardized
                                         tonomous agents and Large Language Models (LLMs), are                   communication protocols, such as Model Context Protocol
                                         increasingly used to address complex, multi-step tasks. The safety,     (MCP) [4], which addresses vertical tool access for a single
arXiv:2510.14133v1 [cs.AI] 15 Oct 2025




                                         security, and functionality of these systems are critical, especially
                                         in high-stakes applications. However, the current ecosystem of          agent, and Agent-to-Agent (A2A) [5] protocol, which ad-
                                         inter-agent communication is fragmented, with protocols such as         dresses horizontal agent coordination via inter-agent delega-
                                         the Model Context Protocol (MCP) for tool access and the Agent-         tion. The MCP allows an agent to discover, authenticate with,
                                         to-Agent (A2A) protocol for coordination being analyzed in              and invoke external tools and data sources in a structured
                                         isolation. This fragmentation creates a semantic gap that prevents      manner. The A2A protocol provides mechanisms for agents
                                         the rigorous analysis of system properties and introduces risks
                                         such as architectural misalignment and exploitable coordination         to discover peer capabilities, negotiate task assignments, and
                                         issues. To address these challenges, we introduce a modeling            coordinate execution across distributed environments.
                                         framework for agentic AI systems composed of two foundational              Turning back to the financial planner example, the host
                                         models. The first, the host agent model, formalizes the top-level       agent must leverage both protocols. It uses the A2A protocol
                                         entity that interacts with the user, decomposes tasks, and orches-      to delegate the subtask of gathering market data from the data
                                         trates their execution by leveraging external agents and tools. The
                                         second, the task lifecycle model, details the states and transitions    agent to the planning agent (horizontal agent coordination).
                                         of individual sub-tasks from creation to completion, providing          Conversely, the transaction agent relies on the MCP protocol
                                         a fine-grained view of task management and error handling.              to invoke an external banking API tool to execute the final
                                         Together, these models provide a unified semantic framework             investment (vertical tool access). The ability of the host agent
                                         for reasoning about the behavior of multi-AI agent systems.             to manage and sequence both A2A and MCP interactions
                                         Grounded in this framework, we define 17 properties for the
                                         host agent and 14 for the task lifecycle, categorized into liveness,    across various agents enables the multi-step tasks to succeed.
                                         safety, completeness, and fairness. Expressed in temporal logic,           The formal or empirical assessment of safety, security,
                                         these properties enable formal verification of system behavior,         and functionality of multi-AI agent systems is increasingly
                                         detection of coordination edge cases, and prevention of deadlocks       important as these systems transition to open-ended, high-
                                         and security vulnerabilities. Through this effort, we introduce the     stakes applications [6], [7]. Here, safety involves prevent-
                                         first rigorously grounded, domain-agnostic framework for the
                                         systematic analysis, design, and deployment of correct, reliable,       ing the agent’s actions from causing unintended or harmful
                                         and robust agentic AI systems.                                          physical or real-world consequences, such as an incorrect
                                                                                                                 trade that causes a substantial loss in the financial planner
                                                                I. I NTRODUCTION                                 example. Functionality ensures that the system accurately and
                                            Agentic AI systems are becoming central to the creation              reliably completes the specified task, such as verifying that
                                         of advanced AI. These systems address complex, multi-step               the financial planner calculates risk correctly and executes the
                                         tasks that surpass the capabilities of any single agent by              investment as requested. Safety and functionality issues might
                                         utilizing multiple autonomous agents. Agents in these systems           happen naturally, e.g., due to design flaws or code errors, but
                                         employ Large Language Models (LLMs) as their primary                    security considers adversarial manipulation and unauthorized
                                         reasoning engine, which enables them to perform task plan-              access that can violate such safety and functionality require-
                                         ning, delegation, and complex decision-making [1]–[3]. These            ments. For instance, a malicious external agent could use a
                                         heterogeneous agents (e.g., agents based on language, vision,           prompt injection attack [8] to force the transaction agent into
                                         robotics, and symbolic planning) dynamically communicate,               transferring funds to an unauthorized account, a clear violation
                                         delegate, and collaborate to complete predefined tasks.                 of both security and safety policies.
                                            To illustrate, we consider an automated financial planner.              Recent work has directly or indirectly started studying
                                         A host agent accepts a natural language request from a user             these issues by examining agents in specific settings, such
                                         to “create a budget and invest $1, 000”. It decomposes this             as adversarial agent behavior [9]–[11] and protocol misuse
                                         into subtasks: a data agent gathers market data, a planning             in task delegation [12]. Other lines of research focus on the
                                         agent calculates risk, and a transaction agent executes the final       compromise or malfunction of a single agent that propagates
                                         investment. These specialized agents interact and exchange              errors, escalates failures, or exposes vulnerabilities across the
                                         data to accomplish the multi-step task.                                 entire system [13]–[16]. Further studies also examine the risk
of using combinations of safe models for misuse [17], the               •   We introduce a host agent model, which provides a
security challenges inherent in tool usage [18], and ensuring               rigorous and domain-agnostic foundation for the sys-
secure delegation and preventing protocol misuse in adversar-               tematic analysis of multi-AI agent systems. This model
ial environments [19], [20].                                                formalizes the top-level agent that interacts with the user,
   The current ecosystem of inter-agent communication and                   decomposes tasks, and orchestrates their execution by
coordination is fragmented and lacks a cohesive formal foun-                leveraging external agents and tools.
dation. Existing protocols (e.g., MCP and A2A) are analyzed             •   Building upon the host agent model, we introduce the
in isolation, which creates a semantic gap that impedes rigor-              first definition of the task lifecycle model. This model
ous reasoning about safety, security, and functional properties             unifies tool-use protocols and inter-agent communication
when both are used simultaneously in complex workflows.                     protocols, establishing a common semantic framework for
This absence of a unified framework, a need that is also                    task initiation, delegation, execution, and completion.
recognized by multi-stakeholder initiatives [21], leaves deploy-
ments vulnerable to emergent coordination failures, including           •   We define critical formal properties for multi-agent AI
deadlocks and privilege escalation, that cannot be adequately               systems, derived from the host agent and task lifecycle
verified or mitigated. Achieving verifiable correctness in these            models. These properties ensure the necessary require-
systems requires a rigorous, unified modeling framework that                ments for safety, security, and functionality are satisfied
can capture the execution behavior of diverse agents across                 regardless of the deployment domain.
heterogeneous protocols.                                                                     II. BACKGROUND
   Our work addresses these challenges by introducing a mod-
eling framework for agentic AI systems that consists of two           A. Agentic AI systems
foundational models. These models represent the conceptual               AI agents are autonomous computational entities that per-
representations of the system’s architecture and behavior:            ceive their environment, reason about goals, and take actions
                                                                      to achieve desired outcomes [22]. Unlike traditional pre-
 1) Host Agent (HA) Model: This model formalizes the
                                                                      programmed software, AI agents exhibit adaptive behavior,
    top-level entity that interacts with the user. The HA is
                                                                      making decisions based on incomplete or uncertain informa-
    responsible for accepting user tasks, decomposing them
                                                                      tion. The field progressed from early symbolic approaches [23]
    into structured subtasks, and orchestrating execution by
                                                                      to systems that use LLMs as their reasoning engines [24], [25].
    leveraging external entities (agents via A2A and tools via
                                                                         LLM-based agents show potential for human-level intel-
    MCP). It acts as both a controller and a monitor, ensuring
                                                                      ligence through access to vast web knowledge [26]. These
    safe delegation and state consistency throughout the task
                                                                      systems process natural language, plan multi-step tasks, use
    lifecycle.
                                                                      tools, generate code, analyze data, and perform complex
 2) Task Lifecycle Model: This model formalizes the dy-
                                                                      reasoning [27]. However, single-agent approaches exhibit lim-
    namic structure of task management, detailing the states
                                                                      itations when tasks become sophisticated and multi-faceted.
    and transitions an individual sub-task undergoes from
                                                                      Complex scenarios require specialized expertise across multi-
    its origin to its completion or failure. It provides the
                                                                      ple domains, concurrent execution of interdependent subtasks,
    fine-grained execution logic necessary for dependency
                                                                      and coordination mechanisms that an individual agent cannot
    management and resilient error handling.
                                                                      manage [1], [28], [29].
   These models are the foundational structure that enables              Agentic AI systems represent a paradigm shift toward
formal verification. Specifically, they capture the discrete states   collaborative networks of specialized agents [30], [31]. In
and transitions of the entire coordination process and provide        these systems, autonomous agents interact, communicate, and
the necessary semantic framework (state space) to express             coordinate their actions to solve problems beyond individual
the formal safety, security, and functional requirements. This        scope [32]. These systems leverage distributed intelligence
unified perspective is required to rigorously reason about ex-        to decompose tasks and allocate them to agents based on
ecution behavior, uncover coordination edge cases, and verify         specialized capabilities and current availability.
properties critical to real-world deployments.
   We categorize the formal properties defined within this            B. Agent Communication and Coordination Frameworks
framework as liveness, safety, completeness, and fairness,               Agentic AI systems require standardized frameworks for
which are essential for verifiable system assurance. For in-          individual agent capabilities and multi-agent coordination.
stance, a safety property guarantees that the transaction agent       These frameworks comprise two complementary aspects: pro-
will not execute an investment before the planning agent has          tocols for agents to interact with external resources and tools,
successfully calculated the risk, thus it prevents inconsistent       and protocols for inter-agent communication. For individual
state or premature execution. Conversely, a liveness property         agents, the challenge is ensuring secure, reliable access to ex-
ensures that once a user submits a request, a final response is       ternal computational resources and tools. Agents must discover
eventually returned. This prevents the financial planning task        capabilities, authenticate services, validate parameters, and
from entering a permanent deadlock or starvation state.               handle errors robustly. For multi-agent coordination, protocols
   In summary, we make the following contributions:                   define how agents discover peer capabilities, negotiate task
assignments, exchange information, and maintain consistency          model. This misalignment makes it difficult to reason about
across distributed execution contexts.                               correctness properties, maintain state consistency, and verify
  The MCP [4] and A2A [5] protocols illustrate the inte-             security guarantees when both protocols are used simultane-
gration challenges that arise from combining individual agent        ously within the same system [11]. The complexity is further
capabilities with multi-agent coordination.                          amplified by the dynamic nature of agents, which may exhibit
Model Context Protocol (MCP): Agent-Tool Integration.                behaviors that violate protocol assumptions [33]. This creates
MCP addresses structured tool access for individual Agentic          a lack of formal guarantees in two areas:
AI [4]. MCP is a client-server protocol that enables a single           • Task Handoff Failures: Transferring a task from an
agent to discover, authenticate with, and invoke external tools           A2A-delegating agent to an MCP-invoking agent is prone
and data sources in a standardized manner. The protocol estab-            to failure. This creates unreliable delegation and coordi-
lishes clear interfaces for capability negotiation, parameter val-        nation, which leads to inconsistent state management and
idation, and result handling, allowing agents to safely integrate         inadequate validation of delegation chains. For example,
with tools, resources, and prompts. MCP’s architecture is a               the host agent may delegate a task via A2A to a specialist
hub-and-spoke model where the agent coordinates connections               agent, but the specialist agent may fail to correctly trans-
to multiple tool providers.                                               late or format the required parameters before invoking
Agent-to-Agent (A2A) Protocol: Multi-Agent Coordina-                      a crucial external tool using the MCP, thus halting the
tion. A2A protocol addresses the complementary challenge                  execution chain.
of inter-agent communication and delegation in multi-agent             •   Inconsistent State Management: Without a unified view,
systems [5]. Unlike MCP’s focus on individual agent-tool                   tracking the state of a multi-protocol task becomes unre-
integration, A2A specifically enables horizontal coordination              liable, which results in inconsistent execution outcomes.
between multiple autonomous agents. The protocol provides                  For instance, in a task where a data agent must first
mechanisms for agents to discover peer capabilities, negotiate             confirm data availability via A2A, a reporting agent might
task assignments, and coordinate execution across distributed              prematurely invoke a secure database tool via MCP be-
environments. A2A operates as an open protocol built upon                  fore the data agent’s A2A confirmation is finalized. This
established standards such as HTTP, Server-Sent Events (SSE),              results in the generation of a report based on incomplete
and JSON-RPC, which allows for seamless integration with                   or unverified data.
existing enterprise infrastructures and promotes interoperabil-
                                                                     Coordination Issues. The absence of tools to validate the
ity across various agentic AI.
                                                                     correctness of cross-protocol interactions creates design flaws
   Central to the A2A protocol is the Agent Card, a metadata
                                                                     that an adversary can exploit.
structure that encapsulates an agent’s capabilities, authenti-
                                                                        • Circular Delegation Loops (Deadlocks): This issue
cation requirements, and communication endpoints. Through
Agent Cards, agents advertise functionalities and establish               occurs when tasks pass indefinitely between agents. For
trust with other agents, enabling dynamic collaboration. The              example, Agent A delegates to Agent B (A2A), which in
protocol’s design emphasizes agentic autonomy, allowing                   turn delegates back to Agent A (A2A), creating a cycle
agents to collaborate in their natural, unstructured modalities           that halts progress.
without shared memory or tools. This approach supports multi-          •  Privilege Escalation: This issue occurs when a malicious
agent scenarios, where agents delegate tasks to specialized               agent exploits delegation chains to gain unauthorized
counterparts, thereby optimizing overall system performance.              access to tools. For instance, Agent A, which lacks direct
                                                                          access to a sensitive MCP tool, delegates an innocent-
  III. C HALLENGES IN C OMPOSITIONAL R EASONING OF                        sounding task to Agent B (A2A), which possesses the
                AGENTIC AI S YSTEMS                                       necessary MCP credentials, thereby using Agent B as a
   The integration of the capabilities provided by the MCP and            proxy for unauthorized access.
A2A protocols within Agentic AI systems creates a compo-                These challenges highlight the critical need for formal
sition of multiple components. These components operate on           modeling frameworks that capture the essential properties
fundamentally different abstractions and assume distinct trust       of integrated agent-tool and agent-agent coordination. Such
models [6]. For instance, MCP treats external tools as trusted       frameworks must be capable of reasoning about cross-protocol
resources with well-defined interfaces, whereas A2A handles          interactions, verifying end-to-end correctness properties, and
potentially untrusted peer agents with varying capabilities and      detecting potential vulnerabilities before deployment in high-
reliability guarantees.                                              stakes environments.
   The resulting lack of a unified semantic model creates two
primary classes of risk: (a) architectural misalignment and (b)               IV. M ODELING THE AGENTIC AI S YSTEMS
exploitable coordination issues.                                        We abstract the formal model for the Host Agent (HA) to
Architectural Misalignment and Semantic Gap. The core                provide a unified view of an agentic AI system. The HA
difficulty raises from the lack of a unified semantic layer          receives natural language requests from users, decomposes
between MCP’s tool-centric model and A2A’s agent-centric             them into structured subtasks, assigns those subtasks to AI
                                                                           Host Agent
             1    User prompt            A              Host Agent Core             D          Communication Layer                              5 RPC
                                                                                                                                               communication
                 1' Clarify intent
                                                                                           { REST }

                                             Dialogue        Session
                                                                                                  API           +
             8 Final Response                                             LLM
                                             Manager         Manager                                                    Security
  User

                                     2                       Build Task            Invoke                                             6
                                                         3                      4 sub-tasks
         Discover External                                     DAG                                                                  Return                          A2A Servers
                                                                                                                                   sub-tasks
              Entities
                                                                          C         Orchestrator
                                                                                                                                    results
         B
                                                                                                               Task                                       MCP Servers
                                                                                                            Aggregate
                                                                                                        7
                                                                                     sub-task Planner       & Resolve

             Registry
                                                                                                                                                               E



Fig. 1. Overview of the host agent model with an Agentic AI system. The model mediates user interactions and coordinates task execution across a
heterogeneous set of external entities, specifically MCP [4] and A2A [5] servers. Internally, the HA employs a modular design comprising the Host Agent
Core ( A ), the Orchestrator ( C ), and the Communication Layer ( D ). The execution lifecycle proceeds as follows: A processes the user prompt and
resolves intent; B enables dynamic discovery of external entities; A , B , and C collaborate on DAG-based sub-task planning. Execution involves secure
task invocation managed by D , followed by aggregation and resolution of results by C before the final response returns to the user. This architecture
supports scalable, interpretable, and compositional collaboration among autonomous AI agents and tools ( E ).



agents or other external entities, and aggregates the results.                           structure to support more fluid and emergent task progressions,
It also oversees execution, monitors progress, and manages                               where state transitions are determined by runtime conditions
exceptions or recovery procedures.                                                       and autonomous agent decisions (e.g., failures and fallbacks).
   We then complement the host agent model with the task                                    We leverage these models to abstract key operational struc-
lifecycle model, which specifies the states and transitions a                            tures of Agentic AI systems. This abstraction provides a
sub-task undergoes from its origin to completion. This model                             foundation for property identification aimed at safe system
offers a fine-grained view of task management and captures                               design through the formalization of task delegation, inter-agent
state transitions and dependencies. Our deliberate separation                            coordination, and execution oversight, detailed in Section V.
of the host agent and task lifecycle models serves two main
purposes. First, it establishes clear abstraction boundaries to
delineate responsibilities; the HA handles orchestration and                             A. Constructing the Host Agent Model
intent resolution, while the task model manages the execution
                                                                                            HA is responsible for interacting with users, understanding
details of individual sub-tasks. Second, this separation allows
integration of new sub-task states to the task model without                             and reasoning through queries, and selecting external entities,
requiring fundamental changes to the host agent model.                                   e.g., tools and AI agents.
                                                                                            Figure 1 illustrates its abstracted model in an Agentic
   We derive these models by synthesizing the operational
                                                                                         system. The primary function of HA is to manage the lifecycle
structures, specifications, and reference implementations of
                                                                                         of complex tasks, from initial reception to completion. This
the unified protocols (Anthropic’s MCP [4] and Google’s
                                                                                         includes initiating a strategic context request, a process where
A2A [5]). This methodology incorporates insights from mod-
                                                                                         the HA uses internal state, historical records, and external data
ular orchestration and dynamic task planning practices for
                                                                                         to gather situational information for accurate task decomposi-
Agentic AI systems [21].
                                                                                         tion, optimal tool selection, and multi-agent coordination.
  The architectural principles of the HA are inspired by
modern orchestration frameworks such as LangGraph [34],                                  Definition of the Host Agent Model (H). We define the host
n8n [35], and OpenAI’s AgentKit [36]. However, these frame-                              agent model H as a tuple: H = (A, E, T , R, C, O, CL , SH )
works typically execute predefined, graph-based workflows.                               where the components are defined as follows:
Our model advances this paradigm by empowering the HA to                                      •    A: The set of all autonomous Agents (e.g., A2A Servers).
dynamically generate and adapt execution plans in response
to prompted user requests. This enables autonomous coor-                                      •    E: The set of all External Entities (EEs), such that A ⊆ E,
dination, moving beyond the constraints of static flowcharts.                                      which includes functional tools (MCP Servers).
Similarly, our task lifecycle model generalizes state manage-                                 •    T : The set of all possible user Tasks, where a task T is
ment concepts found in protocols such as A2A. We extend this                                       a tuple T = (ReqU , RespH ).
  •   R: The Registry, which maintains a mapping from ex-          B. Constructing the Task Lifecycle Model
      ternal entities to their capability profiles, R : E →
                                                                   Registry - B . The Registry maintains a dynamic collection
      P(EE inf o × MAP I ).
                                                                   of external entities (EEs)–including autonomous AI agents
  •   C: The Host Agent Core (HAC), responsible for intent         (i.e., A2A servers [5]), functional tools (i.e., MCP servers [4]),
      resolution IU , C : T × SSM → IU .                           and other service endpoints (contained in E ). These entities
  •   O: The Orchestrator, responsible for task decomposition,     are registered to expose capabilities leveraged during task
      execution management, and final aggregation.                 execution. Each EE requires a capability profile (P(EE inf o ×
                                                                   MAP I )), which details its skills (EE inf o ) and associated API
  •   CL : The Communication Layer (CL), responsible for se-       metadata (MAP I ) necessary for invocation. The Registry sup-
      cure, reliable and protocol-agnostic communication with      ports entity registration, efficient querying, and deregistration
      External Entities in E.                                      to enable system extensibility.
  •   SH : The state space of the host agent, encompassing the        Following intent resolution, the HAC queries the Registry
      overall system state, including network activity and the     ( 2 ) to identify available EEs suitable for fulfilling the request.
      status of all managed sub-tasks.                             We formalize EEs discovery and registration as:
   The Orchestrator’s core function is task decomposition,                    R.discover(EE) → P(EE inf o × MAP I )
mapping a resolved intent IU and available entities E to a
Directed Acyclic Graph (DAG) of sub-tasks D: Odecomp :                 R.register(P(EE inf o × MAP I )) → Success | Failure
IU × P(E) → D. The execution management function, Oexec ,          Orchestrator - C Upon receiving the resolved user intent
maps the DAG through sub-task invocations (via D) to the           (IU ) and a set of relevant EEs from the Registry, the LLM
final results: Oexec : D → Fresults                                constructs a structured execution plan. This plan decomposes
   The host agent model provides the foundational state space      the intent into interrelated sub-tasks organized as a Directed
SH and critical functions (C, O) to define the liveness, safety,   Acyclic Graph (DAG) ( 3 ), which is defined as:
completeness, fairness, and reachability properties in Table I.
   1) Functional Architecture and Execution Sequence: We                                       D = (V, S)
detail the operation of HA by referencing specific components      where v ∈ V is a sub-task node, and S is the set of directed
and their interactions ( A - E ), and execution steps ( 1 - 8 ).   edges indicating precedence constraints or data dependencies.
This defines the role of each component and the data flow             The DAG structure supports parallelism, scheduling, and
from the initial user prompt to the final response.                deterministic execution by capturing dependencies. A directed
Host Agent Core (HAC) - A . The HAC is the first point of          edge (vi , vj ) ∈ S implies that sub-task vj executes only after
contact with users and processes their requests ( 1 ). It serves   sub-task vi successfully completes.
as the central reasoning module and comprises three key sub-          The Orchestrator manages the DAG’s execution. It sched-
components:                                                        ules and invokes sub-tasks ( 4 ) according to the dependency
   • Large Language Model ( LLM ): Central to the HAC ’s           structure to ensure prerequisites are satisfied. After external
      function, the LLM interprets user requests and generates     entities execute, the Orchestrator collects results ( 6 ) and
      coherent responses. The LLM and DM collaborate to            aggregates them ( 7 ) by applying predefined aggregation func-
      resolve the underlying user intent (IU ) upon receiving      tions, merging partial outputs into a unified data structure that
      a request (ReqU ).                                           satisfies the original task requirements. This execution process
                                                                   is primarily defined by:
  •   Session Manager (SM): The SM maintains dialogue
      context across multi-turn interactions and manages user-             D := LLM.Build Task DAG(IU , {EE inf o })
      stored credentials for secure access to authenticated                             O.execute(D) → results
      services. We integrate the SM based on evidence that
                                                                           n=|D|
      contextual management improves decision-making and                    ^
      reasoning in LLM-based systems [7], [37], [38].                              O.aggregate(sub task1 , . . . , sub taskn )
                                                                            i=1
  •  Dialogue Manager (DM): The DM handles conversation
     flow. It works with the LLM to clarify user intent, partic-   Communication Layer (CL) - D The CL ensures secure, re-
     ularly when a request is ambiguous, eliciting necessary       liable, and protocol-agnostic communication between the HA
     information before execution proceeds ( 1’ ).                 and EEs in E . It supports multiple communication protocols,
                                                                   such as REST and gRPC, where a protocol defines the com-
The HA’s context-aware reasoning resolves the user intent
                                                                   munication mechanism and serialization format (e.g., gRPC
(IU ), captured formally as:
                                                                   with Protocol Buffers).
                 HAC(ReqU , StateSM ) → IU                            The CL routes sub-tasks for external invocation ( 5 ) and
                                                                   handles communication using the appropriate protocol and
where StateSM represents the current dialogue and credential
                                                                   payload. The primary interface provided is:
state maintained by the SM. Following intent resolution, the
HAC initiates the subsequent task phases.                                   CL.invoke(EE, protocol, payload) → response
                                                                                                                                                 RETRY
                                                                                                                                             SCHEDULED
                           AWAITING
                                                            DISPATCHING
                         DEPENDENCY                                                                                                           FALLBACK
                                                                                                                   FAILED
                                                                                                                                              SELECTED



          CREATED                             READY                          IN PROGRESS                                                        ERROR




                                                                                                                COMPLETED




                                            CANCELED



Fig. 2. The task lifecycle model that details sub-task state transitions from creation to termination. The lifecycle begins in the CREATED state, proceeds through
AWAITING DEPENDENCY (if prerequisites exist), and moves to READY. Execution transitions either directly to IN PROGRESS (internal handling) or via
DISPATCHING (external delegation). Successful execution leads to COMPLETED. Failures transition to FAILED, enabling recovery via RETRY SCHEDULED
or FALLBACK SELECTED, before terminating in ERROR. Sub-tasks may be CANCELED at any stage.



   The payload contains the sub-task description or API call                             states based on execution outcomes, protocol responses,
parameters. The response includes results, error codes, or                               and recovery policies.
computation artifacts, which the Orchestrator receives. Finally,                    This model’s state space St and transition function δ are
the HAC generates the final response (RespH ) through the                         fundamental to verifying task execution integrity. They provide
LLM and delivers it to the user via the DM ( 8 ).
                                                                                  the basis for the safety invariants (e.g., sequential transition
   This overall sequence demonstrates the system’s intrinsic                      checks) and liveness and fairness properties (e.g., eventual ter-
modularity, its reliance on secure communication channels                         mination/progression to READY state), detailed in Section V.
handled by the CL, and the advanced task orchestration
capability inherent in the Orchestrator.                                          Task and Sub-task Dynamics. In a lifecycle model, a task
                                                                                  represents the high-level user request that serves as the core
   Building upon the HA model, we introduce the task lifecycle
                                                                                  unit of work fulfilled by the HA. A sub-task is a lower-
model to provide a fine-grained perspective on how the HA
                                                                                  level, constituent component into which the HAC decom-
manages and executes sub-tasks. While the HA model (Fig-
                                                                                  poses the main task. Each sub-task has its own lifecycle,
ure 1) outlines the system architecture and orchestration, the
                                                                                  tracked within the parent task’s context. This lifecycle with
task lifecycle model (Figure 2) adds a layer of abstraction by
                                                                                  a defined sequence of states supports fine-grained control for
detailing the states and transitions a sub-task undergoes from
                                                                                  asynchronous execution across distributed EEs.
its origin to completion or failure.
                                                                                  Sub-task State Transitions. The sub-task lifecycle begins in
Definition of the Task Lifecycle Model (L). The task lifecy-
                                                                                  the CREATED state. Sub-task execution then involves three
cle model L defines the state space and transition function for
                                                                                  primary stages. For dependency management, if the sub-task
a single sub-task t, where t is a constituent node in the host
                                                                                  requires other sub-tasks to complete (i.e., incoming DAG
agent’s decomposition graph D. L = (St , s0 , Et , δ) where:
                                                                                  edges exist), it enters the AWAITING DEPENDENCY state
   •   St : The set of discrete states a sub-task can occupy. This                and remains paused until all prerequisites are satisfied. Once
       set is defined as:                                                         dependencies are satisfied, the sub-task transitions to the
                                                       
             CREATED, AWAITING DEPENDENCY, READY,                               READY state. For execution initiation, a sub-task in the READY
               DISPATCHING, IN PROGRESS, COMPLETED,
       St = FAILED, RETRY SCHEDULED,                                              state becomes eligible for execution: the HAC delegates it to
                                                                                  an external entity, transitioning to DISPATCHING, or the HA
                                                       
                FALLBACK SELECTED, CANCELED, ERROR
   •   s0 : The initial state, s0 = CREATED.                                      handles it internally, proceeding directly to IN PROGRESS.
                                                                                  For completion, successful execution (internal or external)
   •   Et : The set of external events and internal conditions                    moves the sub-task to the COMPLETED state, which makes its
       (e.g., dependency satisfaction, external failure signal,                   output available for aggregation and triggers dependent sub-
       timeout) that trigger a state transition.                                  tasks in the DAG.
   •   δ: The state transition function, δ : St × Et → St . This                     Execution is subject to failure and recovery mechanisms,
       function dictates the deterministic movement between                       where an execution failure causes the sub-task to transition
immediately to the FAILED state. The system may then invoke           To formally reason about the safety of the system, we intro-
recovery: the sub-task transitions to RETRY SCHEDULED if            duce the Validation Module (VM). The VM is responsible for
a retry mechanism is invoked. If retries are exhausted and a        enforcing trust and validation constraints on external entities
fallback entity is available, the state changes to FALLBACK         (EEs). We define V M (EE) as the proposition that a given
SELECTED. A sub-task can also be set to CANCELED by                 entity EE has successfully passed all predefined validation
the user or an EE at various points in its lifecycle. If all        processes, including schema validation and behavioral assess-
recovery options fail, the sub-task reaches the ERROR state,        ments. For the formal analysis, we assume the existence of a
which signifies terminal failure.                                   correct and functioning VM.
                V. P ROPERTY S PECIFICATION                           •   Example (Host Agent Model, HP9 ): This property
                                                                          ensures task invocation (CL.invoke) is strictly
   The formalization of the HA and Task Lifecycle models                  conditional on the External Entity (EE) having
provides the foundation for analyzing and verifying critical              completed system validation (V M (EE)). Formally,
system properties. In this context, formal properties are veri-           AG (CL.invoke(EE, protocol, payload) → V M (EE)).
fiable statements about the system’s execution paths, used to
ensure the safety, security, and functionality of an Agentic          •   Example (Task Lifecycle Model, TL6 - State Safety):
AI system’s behavior. We structure these properties into two              A sub-task may only enter the COMPLETED state if its
distinct sets: those captured by the HA model (overall orches-            previous state was IN PROGRESS. This guards against
tration, Table I), and those governing the task lifecycle model           incorrect state transitions, expressed as G((state =
(sub-task state transitions, Table II).                                   COMPLETED) → (previous state = IN PROGRESS)).
   These properties do not operate in isolation; they form a        Ordering (Mutual Exclusion). This property ensures agents
tightly coupled system of interdependent guarantees necessary       avoid simultaneous access to exclusive resources and execute
for correct and reliable function in a multi-AI agent environ-      dependent actions in the correct sequence. The Orchestrator
ment. Both sets of properties are expressed using temporal          must strictly enforce the execution order of sub-tasks across
logics, Computation Tree Logic (CTL) and Linear Temporal            specialized AI agents according to the DAG.
Logic (LTL). These logics allow for the precise, unambiguous
                                                                      •   Example (Host Agent Model, HP10 ): A sub-task is
definition of desired system behavior over time and execution
                                                                          invoked only when it has no uncompleted depen-
paths.
                                                                          dencies, preventing the consumption of intermediate
A. Property Design Principles and Definitions                             dataV that is still being updated. This is expressed
                                                                               n=|D|
   Both models include a set of property categories, each                 as   i=1    AG(CL.invoke(EE, protocol, sub taski )) →
containing multiple specific properties, to establish verifiable          dependencies[sub taski ] = ∅.
guarantees. The core categories used in both tables are live-       Completeness. Completeness captures the guarantee that if a
ness, safety, ordering, completeness, fairness, and reachability.   valid solution or coordinated plan exists, the multi-AI agent
These properties ensure the overall system makes progress,          system will find it. This property is relevant for distributed
avoids undesirable states, achieves its goals, and handles          planning, where incomplete reasoning could leave solvable
reliable resource allocation.                                       tasks unfulfilled due to suboptimal agent coordination.
Liveness. Liveness ensures that an Agentic AI system will             •   Example (Host Agent Model, HP1 2): Every
eventually make progress toward a goal. This property guaran-             user prompt (ReqU ) must lead either to intent
tees that, despite asynchronous actions, inter-agent dependen-            clarification (Clarify Intent) or to task planning
cies, or potential conflicts, the system avoids global deadlock           (LLM.Build Task DAG),         ensuring   the  system
or infinite starvation. This is critical in these systems, where          processes every request and initiates a path toward
a lack of progress in one agent can propagate and result in a             a solution. This behavior is captured as AG(ReqU →
cascading stall of the entire system.                                     EX(LLM.Build Task DAG(IU , {EE inf o }))           ∨
   • Example (Host Agent Model, HP1 ): Every user prompt                  Clarify Intent)).
      (ReqU ) must eventually receive a final response (RespH )
                                                                    Fairness. Fairness requires that all sub-tasks delegated to
      from the host agent. This is expressed in temporal logic
                                                                    agents must eventually transition to a completed or error state,
      as AG(ReqU → AF RespH ).
                                                                    or be explicitly terminated (e.g., via a timeout). This prevents
  •   Example (Task Lifecycle Model, TL1 ): A sub-task that         a single agent’s failure from causing a system-wide deadlock,
      is CREATED must eventually terminate in a defined end         ensuring the overall task can progress.
      state (COMPLETED, ERROR, or CANCELED).
                                                                      •   Example (Task Lifecycle Model, TL11 ): A sub-task in
Safety. Safety ensures that agents never enter globally invalid           the AWAITING DEPENDENCY state with all dependen-
or harmful states, even when operating asynchronously or                  cies satisfied must eventually transition to READY. This
under adversarial conditions. In an Agentic AI system, this               prevents starvation due to dependency satisfaction and is
includes verifying that an agent’s actions do not cause irre-             expressed as AG(state = AWAITING DEPENDENCY ∧
versible policy violations in the collective system state.                dependencies satisfied → AF (state = READY)).
                                                                       TABLE I
     S PECIFICATION OF H OST AGENT F ORMAL M ODEL P ROPERTIES (HP1 –HP17 ). T HESE TEMPORAL LOGIC INVARIANTS (CTL/LTL) DEFINE THE
REQUIREMENTS FOR THE OVERALL SYSTEM ORCHESTRATION . T HE PROPERTIES VERIFY THE INTEGRITY OF THE DELEGATION WORKFLOW, ADHERENCE
 TO SECURITY POLICIES ( E . G ., T RUST A NCHOR SAFETY ), AND RELIABLE PROGRESSION TOWARD TASK COMPLETION ACROSS ALL CORE COMPONENTS
                                    (H OST AGENT C ORE , R EGISTRY, O RCHESTRATOR , AND C OMMUNICATION L AYER ).


                                                                        Liveness

      HP1     Every user prompt eventually receives a response from the HA.
              AG(ReqU → AF RespH )
      HP2     The HAC eventually clarifies the intent of each user prompt.
              AG(ReqU → AF IU )
      HP3     Once intent is clarified, the LLM eventually constructs a Task DAG using the discovered external entities.
              AG(IU → AF LLM.Build Task DAG(IU , {EE inf o }))
      HP4     Whenever a Task DAG is successfully built, all sub-tasks it contains are eventually invoked.
                                                               V                                                   
                                                                   n=|D|
              AG(LLM.Build Task DAG(IU , {EE inf o })) →           i=1     AF (CL.invoke(EE, protocol, sub taski ))
      HP5     Every invoked sub-task eventually produces a result.
              AG (CL.invoke(EE, protocol, sub task)) → AF (CL.return result(sub task))
      HP6     Once all sub-task results are returned via the CL, the Orchestrator eventually aggregates and resolves them.
              Vn=|D|
               i=1     AG(CL.return result(sub taski )) → AF(O.aggregate(sub task1 , . . . , sub taskn ))

                                                                         Safety

      HP7     The Task DAG is constructed by the LLM only after entities have been discovered in the Registry.
              AG(R.entities → AX(LLM.Build Task DAG(IU , {EE inf o })))
      HP8     Sub-tasks are invoked only if the Task DAG D has already been constructed and explicitly includes them.
              AG(CL.invoke(EE, protocol, sub task) → sub task ∈ D)
      HP9     Task invocation is strictly conditional on the EE having completed the system’s predefined validation process and possessing a
              minimum status of reliability.
              AG (CL.invoke(EE, protocol, payload) → V M (EE))
      HP10    Every sub-task in the Task DAG D is invoked only when it has no dependencies on other uncompleted sub-tasks.
              Vn=|D|
                i=1   AG(CL.invoke(EE, protocol, sub taski )) → dependencies[sub taski ] = ∅
      HP11    A response from the HA is returned to the user only after every corresponding sub-tasks have been invoked.
                         V                                                    
                            n=|D|
              RespH →       i=1   AG(CL.invoke(EE, protocol, sub taski ))

                                                                      Completeness

      HP12    Every user prompt leads either to intent clarification or to task planning.
              AG(ReqU → EX(LLM.Build Task DAG(IU , {EE inf o })) ∨ Clarify Intent))

                                                                        Fairness

      HP13    All A2A agent RPC calls eventually receive responses (i.e., do not remain pending indefinitely).
              FAIRNESS(Agent RPC Calls)
      HP14    JSON-RPC calls to MCP servers eventually succeed (i.e., do not remain pending indefinitely).
              FAIRNESS(JSON RPC Calls)
      HP15    Every sub-task invocation eventually yields a result.
              FAIRNESS(CL.return result(sub task))

                                                                      Reachability

      HP16    It is always possible to reach a state in which the Host Agent replies to the user.
              EF (RespH )
      HP17    It is always possible to reach a state in which the LLM builds a Task DAG.
              EF (LLM.Build Task DAG(IU , {EE inf o }))
                                                               TABLE II
   S PECIFICATION OF TASK L IFECYCLE F ORMAL M ODEL P ROPERTIES (TL1 –TL14 ). T HESE TEMPORAL LOGIC INVARIANTS (CTL/LTL) DEFINE THE
 REQUIREMENTS FOR ROBUST SUB - TASK EXECUTION . T HE PROPERTIES VERIFY THE INTEGRITY OF STATE TRANSITIONS , DEPENDENCY SATISFACTION ,
                                             AND RELIABLE FAILURE HANDLING MECHANISMS .


                                                                      Liveness

      TL1     Every sub-task that is CREATED eventually terminates in one of the states: COMPLETED, ERROR, or CANCELED.
              AG(state = CREATED → AF (state = COMPLETED ∨ state = ERROR ∨ state = CANCELED))
      TL2     A sub-task in READY that requires an external entity will eventually transition to DISPATCHING.
              AG((state = READY ∧ external entity needed) → AF (state = DISPATCHING))
      TL3     A sub-task in FALLBACK SELECTED eventually transitions to either DISPATCHING, CANCELED, or FAILED.
              AG(state = FALLBACK SELECTED → AF (state = DISPATCHING ∨ state = CANCELED ∨ state = FAILED))
      TL4     A sub-task in DISPATCHING will eventually reach the IN PROGRESS state.
              AG(state = DISPATCHING → AF (state = IN PROGRESS))

                                                                       Safety

      TL5     A sub-task may only enter DISPATCHING if its previous state was READY.
              G((state = DISPATCHING) → (previous state = READY))
      TL6     A sub-task may only enter COMPLETED if it was previously IN PROGRESS.
              G((state = COMPLETED) → (previous state = IN PROGRESS))
      TL7     Once a sub-task enters ERROR, it remains there and cannot return to an active processing state.
              AG((state = ERROR) → AG(state = ERROR))
      TL8     A sub-task may only enter RETRY SCHEDULED if its previous state was FAILED.
              AG(state = RETRY SCHEDULED → previous state = FAILED)
      TL9     A sub-task that enters CANCELED remains permanently in that state.
              AG((state = CANCELED) → AG(state = CANCELED))
      TL10    A sub-task cannot remain indefinitely in AWAITING DEPENDENCY; it must eventually progress to another state.
              AG(state = AWAITING DEPENDENCY → AF (state ̸= AWAITING DEPENDENCY))

                                                                      Fairness

      TL11    A sub-task in AWAITING DEPENDENCY with all dependencies satisfied eventually transitions to READY.
              AG(state = AWAITING DEPENDENCY ∧ dependencies satisf ied → AF (state = READY))

                                                           Specific Flow/Path Properties

      TL12    A sub-task in FAILED with no available fallbacks transitions either to RETRY SCHEDULED or directly to ERROR.
              AG((state = FAILED ∧ ¬has f allbacks) → (AX(state = RETRY SCHEDULED ∨ AX(state = ERROR)))
      TL13    A sub-task in FAILED with available fallbacks transitions to one of: RETRY SCHEDULED, FALLBACK SELECTED, or ERROR.
              AG((state = FAILED ∧ has f allbacks) → (AX(state = RETRY SCHEDULED ∨ AX(state = FALLBACK SELECTED) ∨
              AX(state = ERROR))))
      TL14    A sub-task in RETRY SCHEDULED transitions to DISPATCHING if the retry policy permits.
              AG((state = RETRY SCHEDULED ∧ retry policy permits) → AX(state = DISPATCHING))



Reachability. Reachability requires that desired joint states                  VI. C ASE S TUDY: V ERIFICATION OF A RCHITECTURAL
or configurations are achievable through the agents’ avail-                        C ONTROLS AGAINST A DVERSARIAL B EHAVIOR
able actions and communication patterns. In multi-AI agent                     The host agent and task lifecycle models provide a frame-
coordination, this involves verifying that the combination of               work to detect, constrain, and mitigate adversarial behaviors in
capabilities and allowed transitions can lead from the current              Agentic AI systems. Here, adversarial behavior is defined as
distributed state to the intended goal without deadlocks or                 any external deviation that compromises correctness, violates
inescapable loops.                                                          system invariants, or exploits control flow inconsistencies.
   • Example (Host Agent Model, HP16 ): It is always pos-                      These models can be leveraged to employ a layered security
      sible to reach a state where the host agent replies to the            architecture, where security constraints are encoded as tempo-
      user. This property is expressed as EF (RespH ).                      ral logic invariants at distinct architectural layers. This struc-
ture enables verification of system-level defenses, including      Control Point 4: Communication Layer and Zero-Trust
secure communication, trust anchoring, intent integrity, and       The CL acts as the secure unified protocol security infras-
failure containment.                                               tructure, and enforces a zero-trust model for all protocol-level
Control Point 1: Host Agent Core (Intent Integrity) Se-            exchanges (REST, gRPC, etc.).
curing the primary human-AI interface, the HAC, serves as             • Formal Requirements: The CL.invoke() operation with

the initial control point. The core architecture is vulnerable          target, protocol, and payload arguments must satisfy non-
to attacks such as prompt injection [14], [39], [40] and                negotiable security policies to ensure that any received
jailbreaks [41] that subvert intent integrity.                          data or instruction is authentic and trustworthy. These
   Our model establishes a security boundary by leveraging              policies include verifiable identity, message integrity, and
the explicit “Clarify Intent” phase as a guard. The verification        confidentiality.
process focuses on ensuring all requests lead to a traceable         •   Verifying Reliability (Fairness, HP13 , HP14 ): The fair-
outcome and enforces intent authenticity.                                ness properties implicitly verify the reliability of this
  •   Verifying Correctness (Completeness, HP12 ): Verifies              infrastructure by requiring that all A2A agent RPC calls
      that every user prompt initiates a clear path toward               and MCP JSON-RPC calls eventually receive responses,
      a solution (clarification or planning), which prevents             ensuring continuous, reliable operation of the communi-
      the core from silently discarding or misinterpreting a             cation channel.
      request. This property is defined as AG(ReqU →                                   VII. R ELATED W ORK
      EX(LLM.Build Task DAG(IU , {EE inf o }))             ∨
      Clarify Intent)).                                            Multi-Agent System Architectures and Formal Models.
                                                                   Traditional Multi-Agent Systems (MAS) research established
  •   Verifying Progress (Liveness, HP2 ): Guarantees the          foundational principles for distributed autonomous systems,
      HAC eventually clarifies the intent of each user prompt      focusing on authorization and secure communication between
      (AG(ReqU → AF IU )), and prevents denial-of-service          agents [16]. The development of Large Agentic AI systems has
      via indefinite ambiguity.                                    introduced new architectural challenges, necessitating specific
                                                                   coordination frameworks. Surveys have provided valuable
Control Point 2: Registry (Trust Anchoring) The Registry
                                                                   taxonomies of agent systems [26], [44], [45]; however, these
functions as the trust anchor for all EE interactions. This
                                                                   works largely describe system categories rather than offering
mitigates the supply chain risk of integrating malicious com-
                                                                   operational models that enable property specification, formal
ponents [42], [43]. Our model ensures the integrity of service
                                                                   reasoning, or verification. Our approach builds upon existing
discovery and registration via codified security properties:
                                                                   architectural insights but advances them by proposing the
  •   Verifying Trust (Safety, HP9 ): Enforces that task           models for the host agent and task lifecycle.
      invocation (CL.invoke) is strictly conditional on the        The Challenge of Integrated Protocol Security. The safety
      EE completing system validation (V M (EE)). This             and security of Agentic AI systems is a major research
      formal verification ensures runtime decisions ad-            domain that directly motivates our formal framework. Security
      here to established trust requirements, preventing           challenges in these systems have been comprehensively docu-
      privilege escalation by unvetted entities. Formally:         mented, identifying vulnerability classes such as coordination
      AG (CL.invoke(EE, protocol, payload) → V M (EE)).            attacks, information leakage, and privilege escalation through
Control Point 3: Orchestrator (Delegation Monitoring) The          delegation chains [6]. This work highlights the cascade failure
Orchestrator serves as a runtime security monitor to enforce       phenomenon, where the compromise of a single agent prop-
verifiable security boundaries during task execution through       agates across the entire system. Industry assessments, such
the dependency DAG structure. This layer directly defends          as the OWASP Foundation’s draft “Top 10 for Agentic AI”,
against coordination-based threats.                                codified these concerns and cite “Unreliable Delegation &
                                                                   Coordination” as a critical risk that directly relates to the
  •   Verifying Integrity and Ordering (Safety, HP10 ): The        heterogeneous protocol integration problem we address [46].
      DAG structure guarantees strict ordering. The property       Protocol-Specific Vulnerabilities. The need for formal veri-
      HP10 ensures task invocation occurs only when all uncom-     fication is compounded by the rapid emergence of vulnerabil-
      pleted dependencies are resolved, and prevents premature
                                                                   ities in key communication protocols. Research on the MCP
      execution and data inconsistency.
                                                                   identified major threats, including installer spoofing, sandbox
  •   Verifying Causal Isolation (Failure Containment): The        escape, and critical gaps in privilege management in real-world
      Orchestrator’s dependency management enforces strict         applications [42], [47]. Furthermore, studies demonstrate that
      causal isolation. This confines adversarial effects within   LLMs can be coerced into executing malicious operations via
      the minimal execution subgraph (e.g., a sub-task will        MCP tools, giving rise to retrieval-agent deception attacks
      not proceed if any dependency state is FAILED or             and systematic security benchmarks [48], [49]. Similarly,
      INVALIDATED). This guarantees fault propagation is           foundational security analysis of the A2A Protocol uncovered
      contained, and supports system liveness and safety.          vulnerabilities in identity management and task exchange
protocols [19]. Key risks in these multi-agent communications                    [3] H. Zhang, W. Du, J. Shan, Q. Zhou, Y. Du, J. B. Tenenbaum, T. Shu,
include ensuring message integrity and agent authentication,                         and C. Gan, “Building cooperative embodied agents modularly with
                                                                                     large language models,” in International Conference on Learning Rep-
with proposed improvements focusing on validating agent                              resentations, 2024.
identities and protecting data exchange to mitigate manipu-                      [4] Anthropic. (2024) Introducing the model context protocol. Accessed:
lation [21]. A critical threat is prompt infection [14], where                       2025-06-25. [Online]. Available: https://www.anthropic.com/news/
                                                                                     model-context-protocol
malicious instructions propagate between AI agents, creating                     [5] G. Cloud. (2025) Announcing the agent2agent protocol (a2a). Accessed:
persistence across tasks. This can enable a single compromised                       2025-06-25. [Online]. Available: https://developers.googleblog.com/en/
log file to trigger a cascading attack, potentially escalating into                  a2a-a-new-era-of-agent-interoperability/
                                                                                 [6] C. S. de Witt, “Open challenges in multi-agent security: Towards secure
a company-wide ransomware incident. Empirical studies con-                           systems of interacting ai agents,” arXiv preprint arXiv:2505.02077,
firm that malicious agents can exploit delegation mechanisms                         2025.
and use coordinated manipulation to subvert system goals,                        [7] S. Han, Q. Zhang, Y. Yao, W. Jin, and Z. Xu, “Llm multi-agent systems:
                                                                                     Challenges and open problems,” arXiv preprint arXiv:2402.03578, 2025.
underscoring the insufficiency of traditional security testing for               [8] Y. Liu, Y. Jia, R. Geng, J. Jia, and N. Z. Gong, “Formalizing and bench-
cross-protocol interactions [10], [12], [17]. This body of work                      marking prompt injection attacks and defenses,” in USENIX Security
confirms the urgent need for a unified semantic framework                            Symposium, 2024.
                                                                                 [9] J. tse Huang, J. Zhou, T. Jin, X. Zhou, Z. Chen, W. Wang, Y. Yuan,
capable of verifying integrated A2A and MCP interactions.                            M. Sap, and M. Lyu, “On the resilience of llm-based multi-agent col-
Formal Verification and Property Specification. Our                                  laboration with faulty agents,” in International Conference on Machine
                                                                                     Learning, 2025.
methodology employs property specification, which is vali-                      [10] P. He, Y. Lin, S. Dong, H. Xu, Y. Xing, and H. Liu, “Red-teaming
dated by a history of research in autonomous systems. Early                          LLM multi-agent systems via communication attacks,” in Findings of
work established security models for distributed AI systems                          the Association for Computational Linguistics, 2025.
                                                                                [11] P. Peigné, M. Kniejski, F. Sondej, M. David, J. Hoelscher-Obermaier,
that focus on the foundational security principles of authenti-                      C. Schroeder de Witt, and E. Kran, “Multi-agent security tax: Trading
cation and authorization [16]. More recently, the focus shifted                      off security and collaboration capabilities in multi-agent systems,” in
to the rigorous application of formal methods to verify system                       AAAI Conference on Artificial Intelligence, 2025.
                                                                                [12] S. R. Motwani, M. Baranchuk, M. Strohmeier, V. Bolina, P. Torr,
correctness and reliability. The importance of this approach for                     L. Hammond, and C. S. de Witt, “Secret collusion among AI agents:
agent coordination protocols is widely recognized [46]. Our                          Multi-agent deception via steganography,” in Neural Information Pro-
use of formal logic and property categorization (e.g., live-                         cessing Systems, 2024.
                                                                                [13] Y. Tian, X. Yang, J. Zhang, Y. Dong, and H. Su, “Evil geniuses: Delving
ness, safety) provides the rigorous foundation necessary to                          into the safety of llm-based agents,” arXiv preprint arXiv:2311.11855,
prove end-to-end correctness, distinguishing our work from                           2024.
empirical studies that demonstrate failure without offering a                   [14] D. Lee and M. Tiwari, “Prompt infection: LLM-to-LLM prompt in-
                                                                                     jection within multi-agent systems,” arXiv preprint arXiv:2410.07283,
mathematically verifiable solution.                                                  2025.
                                                                                [15] D. Zhang, G. Feng, Y. Shi, and D. Srinivasan, “Physical safety and cyber
                         VIII. C ONCLUSION                                           security analysis of multi-agent systems: A survey of recent advances,”
                                                                                     IEEE/CAA Journal of Automatica Sinica, 2021.
   The proliferation of communication and coordination proto-                   [16] Y. Hedin and E. Moradian, “Security in multi-agent systems,” Procedia
cols in Agentic AI systems creates a semantic gap that prevents                      Computer Science, 2015.
the rigorous analysis of system safety, security, and function-                 [17] E. Jones, A. Dragan, and J. Steinhardt, “Adversaries can misuse com-
                                                                                     binations of safe models,” in International Conference on Machine
ality. This paper addresses that gap by introducing a modeling                       Learning, 2025.
framework that consists of two foundational models: the host                    [18] J. Ye, S. Li, G. Li, C. Huang, S. Gao, Y. Wu, Q. Zhang, T. Gui,
agent, which orchestrates user-prompted tasks, and the task                          and X. Huang, “ToolSword: Unveiling safety issues of large language
                                                                                     models in tool learning across three stages,” in Annual Meeting of the
lifecycle, which details sub-task execution. Grounded in this                        Association for Computational Linguistics (Volume 1: Long Papers),
framework, we define 31 formal properties, spanning liveness,                        2024.
safety, completeness, and fairness, expressed in temporal logic.                [19] I. Habler, K. Huang, V. S. Narajala, and P. Kulkarni, “Building a
                                                                                     secure agentic ai application leveraging a2a protocol,” arXiv preprint
These properties enable the formal verification of system be-                        arXiv:2504.16902, 2025.
havior, detection of coordination edge cases, and prevention of                 [20] Q. Li and Y. Xie, “From glue-code to protocols: A critical analysis
deadlocks. Our work provides a domain-agnostic approach for                          of a2a and mcp integration for scalable agent systems,” arXiv preprint
                                                                                     arXiv:2505.03864, 2025.
designing and deploying verifiably safe and reliable Agentic                    [21] E. Neelou et al. (2025) A2as: Standard for agentic ai security.
AI systems. To operationalize this methodology, our future                           Accessed: 2025-10-08. [Online]. Available: https://www.a2as.org/
work will focus on automatically detecting property violations                  [22] S. Russell and P. Norvig, Artificial Intelligence: A Modern Approach.
                                                                                     Prentice Hall Press, 2009.
in coded Agentic AI systems by deriving a formal model from                     [23] N. J. Nilsson, Artificial Intelligence: A New Synthesis.         Morgan
the code and model-checking specified properties against it.                         Kaufmann Publishers Inc., 1998.
                                                                                [24] L. Wang, C. Ma, X. Feng, Z. Zhang, H. Yang, J. Zhang, Z. Chen,
                             R EFERENCES                                             J. Tang, X. Chen, Y. Lin, W. X. Zhao, Z. Wei, and J. Wen, “A survey on
                                                                                     large language model based autonomous agents,” Frontiers of Computer
 [1] J. S. Park, J. O’Brien, C. J. Cai, M. R. Morris, P. Liang, and M. S.            Science, 2024.
     Bernstein, “Generative agents: Interactive simulacra of human behavior,”   [25] Z. Durante, Q. Huang, N. Wake, R. Gong, J. S. Park, B. Sarkar, R. Taori,
     in ACM Symposium on User Interface Software and Technology, 2023.               Y. Noda, D. Terzopoulos, Y. Choi, K. Ikeuchi, H. Vo, L. Fei-Fei, and
 [2] N. Nascimento, P. Alencar, and D. Cowan, “Self-adaptive large language          J. Gao, “Agent ai: Surveying the horizons of multimodal interaction,”
     model (llm)-based multiagent systems,” in IEEE International Confer-            arXiv preprint arXiv:2401.03568, 2024.
     ence on Autonomic Computing and Self-Organizing Systems Companion,         [26] Z. Xi, W. Chen, X. Guo, W. He, Y. Ding, B. Hong, M. Zhang, J. Wang,
     2023.                                                                           S. Jin, E. Zhou, R. Zheng, X. Fan, X. Wang, L. Xiong, Y. Zhou,
     W. Wang, C. Jiang, Y. Zou, X. Liu, Z. Yin, S. Dou, R. Weng, W. Qin,          [48] B. Radosevich and J. Halloran, “Mcp safety audit: Llms with the
     Y. Zheng, X. Qiu, X. Huang, Q. Zhang, and T. Gui, “The rise and                   model context protocol allow major security exploits,” arXiv preprint
     potential of large language model based agents: a survey,” Science China          arXiv:2504.03767, 2025.
     Information Sciences, 2025.                                                  [49] Y. Yang, D. Wu, and Y. Chen, “Mcpsecbench: A systematic security
[27] J. Xie, Z. Chen, R. Zhang, X. Wan, and G. Li, “Large multimodal agents:           benchmark and playground for testing model context protocols,” arXiv
     A survey,” arXiv preprint arXiv:2402.15116, 2024.                                 preprint arXiv:2508.13220, 2025.
[28] G. Li, H. A. Al Kader Hammoud, H. Itani, D. Khizbullin, and
     B. Ghanem, “Camel: communicative agents for ”mind” exploration of
     large language model society,” in International Conference on Neural
     Information Processing Systems, 2023.
[29] W. Chen, Y. Su, J. Zuo, C. Yang, C. Yuan, C.-M. Chan, H. Yu, Y. Lu,
     Y.-H. Hung, C. Qian, Y. Qin, X. Cong, R. Xie, Z. Liu, M. Sun,
     and J. Zhou, “Agentverse: Facilitating multi-agent collaboration and
     exploring emergent behaviors,” in International Conference on Learning
     Representations, 2024.
[30] P. Stone and M. Veloso, “Multiagent systems: A survey from a machine
     learning perspective,” Autonomous Robots, 2000.
[31] A. Dorri, S. S. Kanhere, and R. Jurdak, “Multi-agent systems: A survey,”
     IEEE Access, 2018.
[32] A. Tampuu, T. Matiisen, D. Kodelja, I. Kuzovkin, K. Korjus, J. Aru,
     J. Aru, and R. Vicente, “Multiagent cooperation and competition with
     deep reinforcement learning,” PLoS ONE, 2017.
[33] L. Hammond, A. Chan, J. Clifton, J. Hoelscher-Obermaier, A. Khan,
     E. McLean, C. Smith, W. Barfuss, J. Foerster, T. Gavenčiak, T. A.
     Han, E. Hughes, V. Kovařı́k, J. Kulveit, J. Z. Leibo, C. Oesterheld,
     C. S. de Witt, N. Shah, M. Wellman, P. Bova, T. Cimpeanu, C. Ezell,
     Q. Feuillade-Montixi, M. Franklin, E. Kran, I. Krawczuk, M. Lamparth,
     N. Lauffer, A. Meinke, S. Motwani, A. Reuel, V. Conitzer, M. Dennis,
     I. Gabriel, A. Gleave, G. Hadfield, N. Haghtalab, A. Kasirzadeh,
     S. Krier, K. Larson, J. Lehman, D. C. Parkes, G. Piliouras, and
     I. Rahwan, “Multi-agent risks from advanced ai,” arXiv preprint
     arXiv:2502.14143, 2025.
[34] LangChain. (2025) Langgraph. Accessed: 2025-10-7. [Online].
     Available: https://www.langchain.com/langgraph
[35] n8n. (2025) n8n. Accessed: 2025-10-7. [Online]. Available: https:
     //n8n.io/
[36] OpenAI. (2025) Introducing agentkit. Accessed: 2025-10-08. [Online].
     Available: https://openai.com/index/introducing-agentkit/
[37] S. Yao, D. Yu, J. Zhao, I. Shafran, T. L. Griffiths, Y. Cao, and
     K. Narasimhan, “Tree of thoughts: deliberate problem solving with large
     language models,” in International Conference on Neural Information
     Processing Systems, 2023.
[38] X. Li and X. Qiu, “MoT: Memory-of-thought enables ChatGPT to self-
     improve,” in Conference on Empirical Methods in Natural Language
     Processing, 2023.
[39] Y. Liu, G. Deng, Y. Li, K. Wang, Z. Wang, X. Wang, T. Zhang, Y. Liu,
     H. Wang, Y. Zheng, and Y. Liu, “Prompt injection attack against llm-
     integrated applications,” arXiv preprint arXiv:2306.05499, 2024.
[40] K. Greshake, S. Abdelnabi, S. Mishra, C. Endres, T. Holz, and
     M. Fritz, “Not what you’ve signed up for: Compromising real-world
     llm-integrated applications with indirect prompt injection,” in ACM
     Workshop on Artificial Intelligence and Security, 2023.
[41] A. Wei, N. Haghtalab, and J. Steinhardt, “Jailbroken: How does LLM
     safety training fail?” in Neural Information Processing Systems, 2023.
[42] X. Hou, Y. Zhao, S. Wang, and H. Wang, “Model context protocol
     (mcp): Landscape, security threats, and future research directions,” arXiv
     preprint arXiv:2503.23278, 2025.
[43] Z. Li, K. Li, B. Ma, M. Xu, Y. Zhang, and X. Cheng, “We urgently
     need privilege management in mcp: A measurement of api usage in
     mcp ecosystems,” arXiv preprint arXiv:2507.06250, 2025.
[44] T. Guo, X. Chen, Y. Wang, R. Chang, S. Pei, N. V. Chawla, O. Wiest,
     and X. Zhang, “Large language model based multi-agents: A survey of
     progress and challenges,” in International Joint Conference on Artificial
     Intelligence, 2024.
[45] Y. Xie, C. Zhu, X. Zhang, M. Wang, C. Liu, M. Zhu, and T. Zhu, “Who’s
     the mole? modeling and detecting intention-hiding malicious agents in
     llm-based multi-agent systems,” arXiv preprint arXiv:2507.04724, 2025.
[46] OWASP. (2024) Owasp top 10 agentic ai security risks: Key
     threats and mitigation strategies. Accessed: 2025-08-09. [Online].
     Available: https://www.aicloudgovernance.com/guides-best-practices/
     top-10-agentic-ai-security-risks-key-threats-and-mitigation-strategies
[47] Z. Li, K. Li, B. Ma, M. Xu, Y. Zhang, and X. Cheng, “We urgently
     need privilege management in mcp: A measurement of api usage in
     mcp ecosystems,” arXiv preprint arXiv:2507.06250, 2025.
