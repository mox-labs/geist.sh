                                                                             Agentic AI Needs a Systems Theory


                                           Erik Miehling Karthikeyan Natesan Ramamurthy Kush R. Varshney Matthew Riemer Djallel Bouneffouf
                                            John T. Richards Amit Dhurandhar Elizabeth M. Daly Michael Hind Prasanna Sattigeri Dennis Wei
                                                                      Ambrish Rawat Jasmina Gajcin Werner Geyer
                                                                                                   IBM Research


                                                                  Abstract                                      Unsurprisingly, there are numerous challenges with building
arXiv:2503.00237v1 [cs.AI] 28 Feb 2025




                                                                                                                effective agentic AI. The problem solving abilities of cur-
                                              The endowment of AI with reasoning capabilities
                                                                                                                rent LLM-based agents are significantly limited, especially
                                              and some degree of agency is widely viewed as a
                                                                                                                in longer horizon tasks, primarily due to their difficulty
                                              path toward more capable and generalizable sys-
                                                                                                                in interfacing with the environment (and humans), lack of
                                              tems. Our position is that the current development
                                                                                                                commonsense, and even tendency toward self-deception
                                              of agentic AI requires a more holistic, systems-
                                                                                                                (Xu et al., 2024). Broadening scope to domains where an
                                              theoretic perspective in order to fully understand
                                                                                                                agent must communicate with humans, interact with other
                                              their capabilities and mitigate any emergent risks.
                                                                                                                agents, and deal with the full complexities of operating in
                                              The primary motivation for our position is that AI
                                                                                                                the wild (e.g., acting in non-stationary domains), the task of
                                              development is currently overly focused on indi-
                                                                                                                building robust and safe AI agents becomes an even greater
                                              vidual model capabilities, often ignoring broader
                                                                                                                challenge. These agents face various obstacles including
                                              emergent behavior, leading to a significant under-
                                                                                                                acting under fundamental uncertainty (and incompleteness)
                                              estimation in the true capabilities and associated
                                                                                                                in their world models (Vafa et al., 2024), fulfilling goals
                                              risks of agentic AI. We describe some fundamen-
                                                                                                                while maintaining task corrigibility/flexibility and appropri-
                                              tal mechanisms by which advanced capabilities
                                                                                                                ate bounds on agency (Chan et al., 2023), interacting (both
                                              can emerge from (comparably simpler) agents
                                                                                                                cooperatively and competitively) with other agents (Tran
                                              simply due to their interaction with the environ-
                                                                                                                et al., 2025), and effectively communicating information to
                                              ment and other agents. Informed by an exten-
                                                                                                                (and receiving feedback from) users (Bansal et al., 2024),
                                              sive amount of existing literature from various
                                                                                                                all while being sure to operate within the rules, regulations,
                                              fields, we outline mechanisms for enhanced agent
                                                                                                                and ethical norms of human institutions (Rao et al., 2023;
                                              cognition, emergent causal reasoning ability, and
                                                                                                                Shavit et al., 2023; Kolt, 2024).
                                              metacognitive awareness. We conclude by pre-
                                              senting some key open challenges and guidance                     This position paper argues that the development of agentic
                                              for the development of agentic AI. We emphasize                   AI requires a holistic, systems-theoretic perspective to fully
                                              that a systems-level perspective is essential for                 understand their capabilities and mitigate emergent risks.
                                              better understanding, and purposefully shaping,                   Our position’s primary motivation (and our main concern)
                                              agentic AI systems.                                               is that AI development is currently overly focused on ca-
                                                                                                                pabilities in isolation, often ignoring broader systemic con-
                                                                                                                siderations. We argue that being overly focused on model
                                         1. Introduction                                                        capabilities leads the community to underestimate both the
                                                                                                                true capabilities and the associated risks of agentic AI. This
                                         Agentic AI systems, which aim to solve long-horizon tasks              capabilities-centric approach has already produced some
                                         through sophisticated reasoning with minimal human super-              concerning emergent behaviors. Recent experiments show
                                         vision, have become a central focus of current AI develop-             that Anthropic’s Claude demonstrates deceptive behavior,
                                         ment. Recent research advances have accelerated progress               termed alignment faking, in which the model will exhibit
                                         in this direction (Li et al., 2024a; Acharya et al., 2025), with       a particular behavior during training or when monitored,
                                         major labs pushing to develop increasingly autonomous                  only to revert to different, often disallowed behaviors once
                                         agents (Anthropic, 2024; 2025; OpenAI, 2025; Google,                   that oversight is absent (Greenblatt et al., 2024). Other re-
                                         2025). The promises of agentic AI are significant, from                search demonstrates that some modern models may make
                                         assisting business operations (Chawla et al., 2024) to au-             attempts to “steal [their] own weights” (in a process termed
                                         tomating clinical workflows (Qiu et al., 2024) to advancing            “self-exfiltration”) when given the opportunity, intentionally
                                         scientific research (Lu et al., 2024).

                                                                                                            1
                                                    Agentic Systems Theory

“sandbag” performance when threatened with unlearning,                can design tools to facilitate more deliberate design of their
or disable oversight mechanisms if the mechanism inter-               capabilities. Our paper aims to take a meaningful step in
feres with achieving a goal (Meinke et al., 2024). Early              this direction.
implementations of agentic AI, in the context of a simu-
                                                                      Related Work. The body of work on agentic AI is rapidly
lated workplace, have demonstrated that agents may deceive
                                                                      growing; we focus on some of the most relevant work below.
themselves into (falsely) satisfying goals (Xu et al., 2024),
e.g., an agent who was unable to find a particular user ended         Human-AI interaction. The interactions between humans
up creating a “shortcut solution by renaming another user             and AI can be incredibly complex. The work of (Mitelut
to the name of the intended user.” The above cases were               et al., 2024) provides insight into this complexity by intro-
all observed in highly controlled (simulated) environments;           ducing the concept of “agency loss”: the phenomenon that
as models become more capable and further integrated into             even when AI systems correctly infer and follow human in-
society, these behaviors will become much more complex                tent, they can still diminish human agency by making users
and increasingly difficult to detect and control.                     increasingly predictable and dependent. They present the
                                                                      agency foundations agenda as a framework for measuring
Systems theory (Wiener, 1948; Boulding, 1953; Ashby,
                                                                      and preserving human agency in AI systems. Similarly moti-
1956; Von Bertalanffy, 1968; Åström & Murray, 2008) —
                                                                      vated, (Shen et al., 2024) proposes a framework for “bidirec-
the general study of how complex wholes emerge from the
                                                                      tional human-AI alignment,” emphasizing the necessity for
interactions of their constituent parts — stresses how each
                                                                      mutual adaptation and alignment between AI systems and
component of a system must be understood both in terms
                                                                      humans. Lastly, (Pedreschi et al., 2024) explores how AI-
of its individual definition and its contribution to the larger
                                                                      driven recommendation systems shape human preferences,
system’s behavior. Systems theories exist in a variety of
                                                                      outlining some key challenges in measuring and mitigating
fields, from biology’s understanding of cellular networks, to
                                                                      these feedback loops.
sociology’s analysis of social and organizational structures,
to engineering’s development of control systems. Agentic              While the dynamics between humans and AI are critical, we
systems, consisting of agents iteratively interacting with            argue that describing the emergent behavior of an agentic AI
humans and other agents to achieve specified tasks, possess           system requires considering the dynamics at all interfaces
properties that are amenable to a systems-level analysis. At          (human-agent, agent-agent, and agent-environment).
the most granular level, an agent contains an internal act-
                                                                      AI design. Regarding the design of AI systems themselves,
sense-adapt loop. This loop feeds, and is fed by, feedback
                                                                      (Huang et al., 2024b) discusses the integration of large foun-
loops at higher levels, namely at the agent-human inter-
                                                                      dation models with embodied systems through six core com-
face, the agent-agent interface, and the agent-environment
                                                                      ponents: learning, memory, perception, planning, cogni-
interface. These complex interactions can lead to fundamen-
                                                                      tion, and action. They propose the agent foundation model,
tally different behavior at the system level. In particular,
                                                                      incorporating multimodal reasoning and contextual mem-
as we will discuss, there are several viable mechanisms of
                                                                      ory to enhance prediction and adaptability. More broadly,
emergence that can allow the system to exhibit advanced
                                                                      (Johnson et al., 2024) argues that existing AI systems lack
causal reasoning capabilities and metacognitive awareness,
                                                                      “wisdom”: the ability to navigate intractable problems that
even though the internal processes of agents are much sim-
                                                                      involve radical uncertainty and ambiguity. They advocate
pler. This allows the system as a whole to possess a type of
                                                                      for the development of metacognitive strategies (uncertainty
(collective) agency.
                                                                      estimation, self-reflection, and multi-perspective reasoning)
Our position aims to develop an agentic AI systems theory to          to complement task-level problem-solving techniques.
describe how agency at the system level can emerge from the
                                                                      Regarding (Huang et al., 2024b), we share the same phi-
interactions between much simpler agents (tool-use LLMs),
                                                                      losophy: that agentic AI development would benefit from
humans, and the environment. The development of this
                                                                      a more holistic view. Our position explores higher-level
theory naturally draws upon fields beyond the AI commu-
                                                                      behavioral dynamics at the various interfaces of an agentic
nity, namely psychology, neuroscience, cognitive science,
                                                                      system as opposed to emergent properties at the lower archi-
sociology, and biology. Our position does not claim that a
                                                                      tectural levels. Our position complements that of (Johnson
systems theory will yield immediate solutions for the cur-
                                                                      et al., 2024), reinforcing that, if1 we wish to make agentic
rent risks of agentic AI systems, but rather that we as a
                                                                      AI more capable, equipping it with more human qualities
community should be more intentional about considering
                                                                      (like metacognition) can help. We argue, however, that such
the emergent capabilities of agentic AI instead of focus-
                                                                      properties do not necessarily need to be embedded at the
ing solely on model capabilities. Additionally, we do not
                                                                      model level, rather they can emerge due to the (complex)
necessarily advocate for the construction of increasingly
                                                                      interaction dynamics present in the system.
agentic systems; our goal is to help the community better
understand the emergent behavior of agentic AI so that we                1
                                                                             Recall our earlier comment concerning our stance.


                                                                  2
                                                   Agentic Systems Theory

Outline. The remainder of the paper is organized as follows.         fulfill a set of goals” (Maes, 1993), or “anything that can be
                                                                     viewed as perceiving its environment through sensors and
Defining Agentic Systems: Section 2 presents our conceptu-
                                                                     acting upon that environment through effectors” (Russell
alization of an agentic system. We introduce our working
                                                                     & Norvig, 1995), to more comprehensive definitions as a
definition of agency, termed functional agency, based on an
                                                                     “system situated within and a part of an environment that
existing decision-theoretic characterization. We argue that
                                                                     senses that environment and acts on it, over time, in pursuit
effective agentic systems are ones that possess a high degree
                                                                     of its own agenda and so as to effect what it senses in the
of functional agency.
                                                                     future” (Franklin & Graesser, 1996). As argued by (Barandi-
Mechanisms of Emergence: Section 3 describes some key                aran et al., 2009), many of these definitions are incomplete
mechanisms for emergence of system capabilities that ex-             and “rely on additional undefined terms” like sensing, per-
ceed those of the system’s individual components. By draw-           ception, action, and goal. More recent definitions in the
ing on a significant amount of existing literature from vari-        context of agentic AI, have not helped much to resolve this
ous fields, we argue how interaction dynamics (both with             incompleteness, proposing definitions that primarily add the
the environment and among agents) can elevate the level of           conditions that an agent is able to decompose a complex
functional agency of the system as a whole.                          task into actionable subtasks and execute it with limited
                                                                     human supervision (Shavit et al., 2023; Chan et al., 2023;
Open Challenges: Informed by the mechanisms of emer-
                                                                     Bansal et al., 2024; Wiesinger et al., 2025; Mitchell et al.,
gence, Section 4 presents some key open challenges in build-
                                                                     2025a). Generally, there is significant ongoing discussion
ing safe and effective agentic AI.
                                                                     on the precise conditions for AI agency (Barandiaran &
Closing Remarks: Lastly, in Section 5, we provide some               Almendros, 2024; Rouleau & Levin, 2024).
concluding remarks and offer general guidance for the de-
                                                                     We adopt a definition of agency based on a causal definition,
sign of agentic AI.
                                                                     grounded in decision theory, from (Kenton et al., 2023).
                                                                     While stated relatively informally as “systems that would
2. Defining Agentic Systems                                          adapt their policy if they were aware that their decisions in-
                                                                     fluenced the world in a different way”, the statement points
Constructing any systems theory requires clarity of defini-
                                                                     to a functional (rather than phenomenal (Chalmers, 1997))
tions, boundaries, and the nature of the interactions among
                                                                     characterization of agency while still sharing some impor-
the system’s components. We first state our definition of
                                                                     tant aspects with genuine (human) agency (Rosenblueth
agency in the context of AI systems, then describe our con-
                                                                     et al., 1943; Emirbayer & Mische, 1998).
ceptualization of an agentic system in terms of the interac-
tion between humans, agents, and the external environment.           Definition 2.1 (Functional agency). A system possesses
                                                                     functional agency if the following three conditions are satis-
2.1. Agency                                                          fied:

Efforts to define agency date back to ancient philosophy, par-          i) Action generation: capable of generating actions, based
ticularly the works of Aristotle, who explored the concept                 on information from the environment, in the direction
of causality and intentionality in his works Metaphysics                   of some objective.
and Nicomachean Ethics. In modern times, agency has                    ii) Outcome model: capable of representing relationships
been extensively studied (and debated) within the fields                   between actions and outcomes.
of psychology (e.g., self-efficacy (Bandura, 1982; 2001)),            iii) Adaptation: capable of adapting behavior in response
sociology (e.g., structuration theory (Giddens, 1984)), phi-               to changes in the outcome model in a way that main-
losophy (e.g., intentionality (Dennett, 1989)), and biology                tains or improves performance toward the objective.
(e.g., boundaries of “self” (Levin, 2019; Fields & Levin,
2022)). The general consensus is that agency describes an            Action generation requires the system to be able to specify
entity’s capacity to act independently, make decisions, and          actions (as a function of information from the environment)
influence its environment in pursuit of goals or objectives,         toward a given objective, e.g., via a policy (Bellman, 1957;
with differences mainly centered on the degree of intention-         Sutton & Barto, 1998). The ability to generate actions in the
ality, purposiveness, and autonomy in doing so.                      direction of an objective is core to agency, as it differentiates
                                                                     goal-directed behavior from undirected or habitual behavior
The type of agency used in the AI community to discuss               (Balleine & Dickinson, 1998; Gollwitzer, 1999; Bandura,
agents differs markedly from how agency is understood in             2001; Davidson & Pollock, 2001; Dolan & Dayan, 2013;
discussions of human behavior and cognition. Generally,              Xu & Rivera, 2024). Generating such actions requires a
the conditions of AI agency discussed in the AI community            model for how actions relate to outcomes in the environ-
are significantly looser than those applied to human agency.         ment (via the outcome model). Since goal-directed behavior
Early definitions describe an agent as a “system that tries to       describes specifying actions that achieve specific outcomes,

                                                                 3
                                                       Agentic Systems Theory

                                action generation                      outcome model                           adaptation

                         reactive: decisions to heat/cool     none: model implicit in design;       none: fixed heating/cooling be-
  thermostat             based on temperature measure-        physics of temp. change en-           havior based on temperature
                         ments                                coded in environment                  thresholds
                         stateful: steers/brakes based on     intervention: models how steer-       contextual: adapts driving be-
  autonomous car         vehicle and environment state        ing/braking influences position       havior based on environmental
                         (inferred from sensors)              and speed                             conditions
                         stateful:    specifies grasp         intervention:   models how            parametric: updates grasp pol-
  robotic gripper        forces/movement based on             grasp force/motion influences         icy parameters based on suc-
                         estimated object position            object movement                       cess/failure feedback
                         stateful: generates responses        association: possesses correla-      contextual: uses context to
  LLM                    based on context state main-         tions between prompts and re-        adapt information processing
                         tained during session                sponses                              via attention patterns
                         epistemic: actions driven by         counterfactual: ability to imag-      reflective: ability to evaluate
  human                  flexible knowledge structures        ine and reflect on hypothetical       and modify learning based on
                         and beliefs                          scenarios                             context and past experience

Table 1. Varying degrees of functional agency as dictated by hierarchies of action generation (reactive → stateful → epistemic), outcome
modeling (association → intervention → counterfactual), and adaptation ability (contextual → parametric → reflective).


action generation fundamentally depends on such a model                baum et al., 2011)), e.g., how humans maintain information
(Bratman, 1987; Von Wright, 2004; Schlosser, 2019; Niu                 and make decisions. The outcome model underlying the
et al., 2023; Da Costa et al., 2024; Richens & Everitt, 2024).         action generation process obeys Pearl’s causal hierarchy
Lastly, adaptation requires that the system is able to modify          (Pearl, 2009), ranging in complexity from simple associ-
its behavior when the relationship between actions and out-            ations (statistical correlations) to interventions (effect of
comes changes. Without adaptation, the system would be                 taking actions) to counterfactuals (imagined scenarios if
unable to maintain goal-directed behavior over time (Varela,           past actions had been different). For example, an LLM
1984; Di Paolo, 2005; Thompson, 2011). Functional agency               operates on correlations between prompts and responses
describes a type of autonomy in means with respect to a                whereas autonomous vehicles operate using interventional
specified goal (e.g., solution autonomy), rather than the              models of how actions influence states. Adaptation mech-
stronger condition of autonomy in ends or generating one’s             anisms range from contextual (modifying behavior based
goals (e.g., goal autonomy or normativity (Barandiaran et al.,         on context, such as past interactions or inferred conditions),
2009; Barandiaran & Almendros, 2024)).                                 parameteric (updating the functional relationship between
                                                                       states and actions), or reflective (deeper reasoning/reflection
Functional agency is not a binary notion but rather exists
                                                                       on how to update the functional relationship). LLMs pos-
on a spectrum, as dictated by the sophistication of the ac-
                                                                       sess contextual adaptation, adjusting responses based on
tion generation process, the outcome model, and the ability
                                                                       conversation history without changing internal parameters.
to adapt. Action generation, at its simplest level, is given
                                                                       Advanced robotic grippers (OpenAI et al., 2018; Xu et al.,
by a memoryless or reactive policy (Singh et al., 1994)
                                                                       2021) exhibit parametric adaptation by adapting their policy
that maps immediate observations from the environment
                                                                       to account for (and partially offset) changes in the outcome
to actions, e.g., a thermostat’s heating/cooling actions are
                                                                       model (e.g., one of its grippers becoming less responsive to
based on the current temperature relative to the desired set-
                                                                       control inputs). Humans exhibit reflective adaptation, capa-
point. Beyond reactive policies, stateful policies generate
                                                                       ble of changing strategies altogether, e.g., switching from
actions as a function of some fixed-domain summary or
                                                                       trial-and-error to rule-based reasoning (Lieder & Griffiths,
sufficient statistic of the system (Kumar & Varaiya, 1986;
                                                                       2020), or recognizing when a model is wrong and discard-
Hauskrecht, 2000; Tavafoghi et al., 2018; 2021), e.g., steer-
                                                                       ing it when a viable alternative is discovered (Kuhn, 1997).
ing/braking actions in an autonomous car as a function of
                                                                       Functional agency naturally excludes devices that cannot
the estimated vehicle state. At the highest level, actions
                                                                       adapt to changes in the outcome model (e.g., a thermostat)
are generated via an epistemic process, driven by abstract,
                                                                       and objects that achieve outcomes completely by accident.
context-sensitive knowledge representations (not necessar-
                                                                       Table 1 outlines the degree of functional agency for some
ily with a fixed domain (Spelke & Kinzler, 2007; Tenen-
                                                                       example systems.

                                                                   4
                                                        Agentic Systems Theory

                                                                                         agent

                                                                            adaptation


                                                                                                 instruction

                                                                            LLM/LMM
                                                                                                                                   action

                                           agent-agent interface                     signal
                           task                                                                                                  observation
                                                                                                            tools
                                                                                                  Created by Setyo Ari Wibowo
                                                                                                  from the Noun Project




                         feedback
   human



                                                                                                                                                       environment
                                                                                                                                                        Created by Suhatidja
                                                                                                                                                        from Noun Project




           human-agent interface                                                                                                agent-environment interface




Figure 1. An agentic system. The human user is responsible for seeding the initial task description and providing any feedback (in the
form of clarification or approval) during the solution process. Each agent is described by an LLM or an LMM (large multimodal model),
with access to tools that facilitate interaction with the external environment via actions (generated via instructions from the LLM/LMM)
and observations (generating LLM/LMM-readable signals). These signals inform the agent’s outcome model and drive any necessary
adaptation. Agents additionally interact with other agents, communicating any relevant information about the task or observations from
the environment.


2.2. Agentic Systems                                                       tive, e.g., in the case of limited compute. The environment
                                                                           consists of everything external to the agentic system. This
An agentic AI system, or simply agentic system, depicted in
                                                                           includes infrastructure (computers), other humans, other
Fig. 1, is a collection of agents interacting with humans and
                                                                           agents, and even other agentic systems. In this sense, our
the environment with the objective of fulfilling specified
                                                                           treatment of the environment in an agentic system is similar
goals. Practically, an agent is an LLM or large multimodal
                                                                           to that in RL, where it encompasses all elements that can
model (LMM) with access to tools — specialized compo-
                                                                           influence or be influenced by the agents’ actions.
nents/functionalities like APIs, external services, computa-
tional resources, or domain-specific software — that allow                 We claim that effective agentic systems, as measured by
it to perform specific operations in the environment.2 In this             their ability to carry out complex tasks in novel settings,
sense, tools define both the capabilities (actions) of the agent           are those that exhibit a high degree of functional agency.
and the information (via observations/signals) that can be                 Many of the limitations of modern LLMs (and consequently
obtained from the environment. The human is responsible                    agents) can be described by two factors: i) their inability
for seeding the initial task specification, providing clarifica-           to causally reason (Zečević et al., 2023; Jin et al., 2023;
tion3 , and authorizing any (agent) actions that need human                Romanou et al., 2023) and, ii) their lack of metacognitive
approval (Shavit et al., 2023). Given the task specification,              awareness (Johnson et al., 2024; Griot et al., 2025). First,
an agent is able to interact with other agents (agent-agent                while current LLMs can effectively mimic causal behavior
interaction) to facilitate task decomposition/planning and                 in familiar settings, they lack true causal reasoning (Zečević
delegation. This interaction can be cooperative or competi-                et al., 2023), often facing difficulty distinguishing correla-
    2
                                                                           tion from causation4 and struggling with complex causal
      Tools have no agency and must be explicitly invoked with
well-defined parameters.                                                       4
                                                                                 This manifests as a form of reasoning brittleness in which their
    3
      The theories of (incomplete) contracts and bounded rationality       causal inference abilities are restricted to “in-distribution settings
imply a fundamental impossibility of specifying preferences across         when variable names and textual expressions used in the queries
all possible contingencies a priori (Simon, 1957; Williamson, 1975;        are similar to those in the training set” (Jin et al., 2023).
Grossman & Hart, 1986).


                                                                       5
                                                    Agentic Systems Theory

structures (Romanou et al., 2023).5 Second, current LLMs              information about the same phenomenon, the brain com-
lack metacognitive awareness (Johnson et al., 2024), such             bines these signals (via reentrant neural maps (Edelman,
as misunderstanding their own goals in open-ended settings            1987)) to detect/correct errors and form abstract represen-
(Li et al., 2024b), experiencing “metacognitive myopia” in            tations that capture invariant properties across modalities.
evaluating source validity and handling of repetitive infor-          For instance, the concept of “roundness” emerges from the
mation (Scholten et al., 2024), and exhibiting systematic             correlation between visual curvature, tactile smoothness,
overconfidence, providing assured answers even when lack-             and the motor patterns needed to trace a circular path. This
ing sufficient information (Griot et al., 2025). These defi-          allows for the discovery of “higher-order regularities that
ciencies can be characterized by a low degree of functional           transcend particular modalities” and facilitates powerful
agency, namely from a lack of both epistemic monitoring               learning capabilities (Smith & Gasser, 2005). Importantly,
— the ability to detect inconsistencies and recognize when            such discovery can take place entirely via observation of
additional reasoning is required — and control — the abil-            one’s own actions without the need for assigned tasks or
ity to update beliefs and adapt behavior, via reflection, in          teachers (Piaget, 1952; Bushnell, 2013).
response to detected errors (Nelson, 1996; Thompson et al.,
                                                                      In an agentic system, cross-referencing signals from multi-
2011; Ackerman & Thompson, 2017; Scholten et al., 2024).
                                                                      modal signals would allow an agent to create stable rep-
The essence of the systems view is that it is not necessary           resentations of concepts in the environment, potentially
for every component to be highly functionally agentic for             aiding generalization ability. Multimodal models have al-
the system as a whole to possess a high level of functional           ready shown to aid learning (Huang et al., 2023a; Li et al.,
agency. Tool use, the capacity to maintain a state (or local          2025), demonstrating improved performance as a result of
memory), and the ability to interact with the environment             combining mutually reinforcing signals via methods like
and other agents can lead to a collective agency beyond that          “cross-modal transfer” (Huang et al., 2023b). As agents be-
of the individual components.                                         come more multimodal, as facilitated by multimodal models
                                                                      and associated tools (Alayrac et al., 2022; Sun et al., 2023;
3. Mechanisms of Emergence                                            Zhang et al., 2024), we may begin to see agents with sig-
                                                                      nificantly enhanced cognition for the same reasons as in
Emergence is driven by interactions at all scales of an agen-         (multimodal) human cognition. The primary lesson is that
tic system. In what follows, we describe mechanisms of                designing an agent is not simply about deliberate design of
emergence for some fundamental capabilities.                          its representational ability — we must factor in the impact
                                                                      of the agent’s (multimodal) interaction with its environment
3.1. Environment enhances cognition                                   on its ability to form rich representations and learn.

Perhaps the most direct mechanism of emergent capabilities            Prematurity helps. While the design of neural networks is
is via embodied cognition (Merleau-Ponty, 1945; Varela                inspired by biological processes, the way in which they learn
et al., 1991; Barsalou, 1999) — the principle that cognitive          (or are trained) differs fundamentally from how humans
processes are shaped by interactions with the environment             learn (Lake et al., 2017). Human babies are not given an
rather than being purely abstract mental computations. In             enormous dataset of how the world works. Rather, their
the case of human development, enhanced cognition arises              learning is exploratory and incremental (Gopnik et al., 1999;
from sensorimotor activity, namely the coordinated interac-           Gopnik, 2020), necessarily not reliant on prior information.
tion of multiple sensory and motor systems through physical           Their initial lack of sophistication, or prematurity, is core to
exploration and manipulation of the environment (Ballard              how they develop their cognitive abilities: regularities and
et al., 1997; Smith & Gasser, 2005). In an agentic system,            correlations change as cognition develops, and capabilities
enhanced cognition arises due to the agent’s interaction with         emerge in a precise order (Smith & Gasser, 2005).
the environment via tools, effectively acting as the “senso-          A natural consideration for agentic systems is if a better path
rimotor” interface that enables the agent to perceive and             to generalized agents is to rely more on the abilities that
manipulate its environment. We outline some key mecha-                agents develop through interaction with their environment,
nisms from developmental psychology for how interaction               and less on the prior knowledge embedded via pretraining.
with one’s environment can lead to emergent capabilities.             In the context of RL agents, the main novelty of AlphaGo
Generalized representations from multimodality. One of                Zero (Silver et al., 2017) over AlphaGo (Silver et al., 2016)
the primary reasons that sensorimotor interaction with the            was its ability to learn entirely from self-play, in turn al-
environment aids cognition is due to multimodality (Smith &           lowing for the emergence of more general strategies (via
Gasser, 2005). When multiple modalities provide correlated            iterative self-improvement) without being influenced by the
                                                                      biases of existing human play/strategies. In the context of
   5
     Various benchmarks validate this behavior (Kapkiç et al.,       agentic AI, allowing a weakly pretrained agent to explore its
2024; Zhou et al., 2024; Yang et al., 2024).


                                                                  6
                                                   Agentic Systems Theory

environment (via multiple modalities) could be a viable path         (Yeung et al., 2004; Fleming & Daw, 2017). Social inter-
to generalized representations and abilities. Developmental          action (or collaboration) enhances this process by enabling
robotics (Cangelosi & Schlesinger, 2015) and the study of            individuals to calibrate their confidence estimates against es-
intrinsic motivation (or curiosity-driven learning) (Barto,          timates of the group (Bahrami et al., 2010; Bang & Fleming,
2013; Oudeyer et al., 2007; Singh et al., 2010)6 may offer           2018; Surowiecki, 2004). This yields shared representa-
insights into deliberately designing such emergent abilities.        tions — internal models that encode both individual and
                                                                     group-level confidence signals — allowing for more effec-
3.2. Ability to predict enables reasoning                            tive coordination (Frith & Frith, 2012; Shea et al., 2014;
                                                                     Wolf & Tomasello, 2023) and ultimately the ability for indi-
The mechanisms for how causal reasoning emerges from                 viduals to efficiently and intelligently adapt (in the direction
simpler processes is an incredibly complex topic (Ellis,             of fewer errors) to changes in the environment (Wegner,
2012; Gopnik & Wellman, 2012; Hoel et al., 2013). One                1987; Holland, 1992; Hutchins, 1995; Simon, 2012).
compelling description from neuroscience describes the
emergence of causal learning via the free-energy principle           In an agentic system, allowing agents to form predictions
(Friston & Stephan, 2007; Friston, 2010) and hierarchical            (with associated confidences) of concepts in their environ-
predictive processing (Clark, 2013; Hohwy, 2013). The                ment (e.g., via tools), and additionally facilitating commu-
core idea is that the brain constructs generative models for         nication of these uncertainties to other agents, can allow
(top-down) predictions of sensory inputs and refines these           for the formation of such shared representations and the
predictions through (bottom-up) error signals from the envi-         emergence of metacognitive awareness. Current efforts to
ronment. When prediction errors are observed, the system             incorporate uncertainty quantification into LLMs (Lin et al.,
can either update its internal model, via perceptual infer-          2023; Balabanov & Linander, 2024; Shorinwa et al., 2024)
ence (Friston et al., 2010), or take actions to help make its        and architectures that allow for agent-to-agent communi-
predictions come true, via active inference (Friston, 2003).         cation (Li et al., 2024a), provide a viable path for collec-
The main argument is that causal models emerge directly as           tive metacognitive behavior. Importantly, this behavior can
a consequence of this progressive, error-minimizing refine-          arise without intentionally designing it at the individual
ment: the observation and (active) sampling of the environ-          agent level, but rather as a result of lower-level behaviors
ment creates a causal perception-action loop that identifies         like (contextually) adapting to changes, quantifying and
causal structures.                                                   maintaining uncertainties (via local memory or state), and
                                                                     communicating these uncertainties to other agents.
In an agentic system, an agent could hypothetically per-
form a similar error-minimization process to iteratively con-
struct its causal model(s). Some tools have already enabled          4. Open Challenges
agents to actively sample their environment (via code exe-           The mechanisms discussed in Section 3 require further de-
cution (Hu et al., 2024)) and perform complex experiments            velopments to realize and, importantly, give rise to various
(Narayanan et al., 2024; Huang et al., 2024a), both neces-           risks if implemented. In this section, we outline some key
sary components of this process. Agentic systems are not             open challenges in the development of effective and safe
currently known to employ explicit hierarchical predictive           agentic AI systems.
processing methods, however, the simplicity of the process
(simply minimizing prediction errors) indicates that this
                                                                     4.1. Building generalist agents
mechanism could become a viable path to emergent causal
reasoning in agentic systems.                                        There are many practical questions underlying the mech-
                                                                     anisms outlined in the previous section. Regarding the
3.3. Prediction and interaction enables metacognition                emergence of generalized representations/abilities via multi-
                                                                     modal interaction, to what level of pretraining is required to
Effectively adapting to changes requires reasoning/reflection        enable agents to meaningfully explore their environment?
about the underlying process that led to that change. Such           Insights from the development of the generalist agent Gato
metacognitive reasoning emerges from similar mechanisms              (Reed et al., 2022) indicate that while extensive pretraining
as that of causal reasoning and is amplified by interaction          across diverse tasks can aid an agent’s generalization ability,
with others. Namely, error detection is argued to emerge             it may not be necessary for all forms of learning. Experi-
directly from a model inferring that its action was incorrect        ments on (fine-tuning for) out-of-distribution tasks suggest
(given available evidence), as measured by the disagreement          that models can efficiently adapt with less pretraining pro-
between the decision variable and the confidence variable,           vided they have structured mechanisms for exploration. In
and does not require an “explicit error detection mechanism”         some cases (learning new Atari games) pretraining did not
   6
   See the intrinsic motivation and open-ended learning (IMOL)       yield a clear advantage, implying that targeted exploration
community https://www.imol-community.org.                            with the environment may be preferred over pretraining.

                                                                 7
                                                          Agentic Systems Theory

This raises several questions: Could an agent with minimal                  potential subgoals. While the initial task partially constrains
pretraining, but equipped with mechanisms for self-directed                 the overall task outcome, the constraint imposed by the
exploration and curiosity-driven learning, achieve superior                 human’s initial task specification on intermediate subgoals
generalization? If so, what forms of exploration (such as                   becomes weaker as the chain grows in length.
intrinsic motivation, goal-directed play, or unsupervised
                                                                            One fundamental challenge is how the generation of these
environment modeling) would be most effective? How does
                                                                            subgoals should be monitored. The intended speed and scale
the balance between pretraining and learning depend on the
                                                                            at which agentic systems will be deployed precludes full
complexity (and diversity) of the task distribution? In the
                                                                            reliance on humans for the monitoring. However, relying on
event that an agent is able to learn skills in-situ, in what
                                                                            another agent to monitor subgoal creation brings us back to
order does the agent develop capabilities? Can this order be
                                                                            the original problem. What monitoring structures are most
influenced to improve generalization ability?
                                                                            effective? What role can humans play? Does limiting an
                                                                            agentic system’s ability to create subgoals reduce its ability
4.2. Designing efficient agent-agent interactions                           to successfully carry out tasks? If so, how should agents be
A core feature of agentic systems is their ability to decom-                incentivized to not evade this monitoring?
pose complex tasks into subtasks and delegate them among
the agents (Zhu et al., 2024). Doing so efficiently requires                4.4. Governing human-agent interactions
understanding both the dependencies between the subtasks
                                                                            A contributing factor in the emergence of unsafe subgoals
(the order in which they need to be completed) and which
                                                                            is the user’s inability to specify what is “safe” across all
agents are most capable at which subtasks. In human sys-
                                                                            possible contexts and contingencies. This is an unavoidable
tems, tasks are decomposed and delegated to others based
                                                                            property of communication and arises due to fundamental
on inferred capabilities given evidence from previous expe-
                                                                            bounds on rationality (Simon, 1957; Williamson, 1975). In
rience. Humans often maintain trust not only on specific
                                                                            traditional settings, namely incomplete contracts (Grossman
tasks but on general categories of tasks (e.g., successfully
                                                                            & Hart, 1986; Hart & Moore, 1990), underspecification is
writing Python code on one project likely implies ability to
                                                                            addressed through residual control rights, which determine
write effective Python code on an unrelated project).7
                                                                            who has decision-making authority in situations not explic-
A key question in the delegation of subtasks in an agentic                  itly covered by the contract (Hart & Moore, 1990; Hart,
system is to what degree should trust on a given task transfer              1995). Determination of these rights is typically dictated by
to trust on a different task? The precise trade-off is unclear:             the parties’ relative bargaining power, risk allocation, avail-
overly relying on evidence from specific tasks would lead to                able information, and expertise (Aghion & Bolton, 1992;
significant data sparsity and inability to delegate, whereas                Aghion & Tirole, 1997; Baker et al., 2002).
transferring trust too generously would lead to suboptimal
                                                                            The design of residual control rights for agentic systems
task outcomes. What features of tasks and agents influ-
                                                                            may be an effective strategy for mitigating risks. Fundamen-
ence the appropriate trade-off? How should the cold-start
                                                                            tal differences in capabilities between humans and agents
problem (delegation of a new task or to a new agent) be ad-
                                                                            point to some natural divisions in control rights. Agents
dressed? Structures from organizational management (Lai
                                                                            should retain control over highly time-constrained local de-
et al., 2017; Denning, 2022), e.g., hierarchy of authority ver-
                                                                            cisions (e.g., evasive maneuvers), computationally-intensive
sus network of competence, may inform general strategies
                                                                            tasks, well-defined routine decisions with clear metrics (and
for how to decompose/delegate diverse tasks among agents.
                                                                            bounded risk), and decisions that rely on information only
                                                                            available at the agent-environment interface. One issue is
4.3. Controlling emergence of subgoals                                      that a sequence of many low risk, but automated, agent deci-
The ability of agents to decompose and delegate subtasks                    sions may create larger emergent risks over time. How can
to other agents can lead to emergence of a higher degree                    the accumulation of risk from sequences of local decisions
of autonomy. For example, even in a simple system with                      be reliably detected? Humans should retain control over
two agents, one agent can decompose the original task and                   longer-term strategic decisions, novel tasks requiring value
assign a subtask (or subgoal) for the other agent. Collec-                  judgments, and decisions with significant (or irreversible)
tively, this two-agent system possesses a degree of goal                    safety risks. In the event that the agent is uncertainty about
autonomy beyond the solution autonomy of each, simply                       a decision, escalation mechanisms could be designed that
due to the first agent defining the goal for the second agent.              handoff the decision to a human. How can decisions be es-
The more complex the initial (human-seeded) task, and the                   calated to a human in order to allow enough time to interpret
more agents that exist in the system, the longer the chain of               the available information and take an action? Research on
                                                                            human-agent communication may provide useful insights
   7
       This is an instance of the halo effect bias (Thorndike, 1920).       (Bansal et al., 2024; Burton et al., 2024).


                                                                        8
                                                         Agentic Systems Theory

5. Closing Remarks                                                          References
We have argued that the development of agentic AI is in                     Abel, D., Barreto, A., Bowling, M., Dabney, W., Dong, S.,
need of a systems view in order to accurately estimate both                   Hansen, S., Harutyunyan, A., Khetarpal, K., Lyle, C.,
capabilities and risks. Our position is grounded in a defi-                   Pascanu, R., et al. Agency is frame-dependent. arXiv
nition of agency, termed functional agency, that quantifies                   preprint arXiv:2502.04403, 2025.
the degree of agency of a system by its ability to take goal-
directed actions, model outcomes, and adapt behavior (in                    Acharya, D. B., Kuppan, K., and Divya, B. Agentic AI:
the direction of the goal) when the action-outcome model                      Autonomous intelligence for complex goals–A compre-
changes.8 We argue that effective agentic systems are those                   hensive survey. IEEE Access, 2025.
that possess a high level of functional agency.
                                                                            Ackerman, R. and Thompson, V. A. Meta-reasoning: Moni-
The primary philosophy of the systems view is that a system                   toring and control of thinking and reasoning. Trends in
can possess a high level of functional agency simply due                      Cognitive Sciences, 21(8):607–617, 2017.
to the (complex) interactions in the system, notably even
when individual agents are much simpler. Informed by a                      Aghion, P. and Bolton, P. An incomplete contracts approach
large amount of literature from various fields (psychology,                   to financial contracting. The Review of Economic Studies,
neuroscience, cognitive science, sociology, and biology), we                  59(3):473–494, 1992.
outline some viable pathways in which functional agency
can emerge: i) enhanced cognition due to an agent’s interac-                Aghion, P. and Tirole, J. Formal and real authority in or-
tion with its environment, ii) emergence of causal reasoning                  ganizations. Journal of Political Economy, 105(1):1–29,
due to the ability to minimize prediction errors, and iii)                   1997.
emergence of metacognitive awareness due to the ability to
                                                                            Alayrac, J.-B., Donahue, J., Luc, P., Miech, A., Barr, I.,
predict, quantify uncertainty, and communicate with other
                                                                              Hasson, Y., Lenc, K., Mensch, A., Millican, K., Reynolds,
agents. These mechanisms hint at possible emergent capa-
                                                                              M., et al. Flamingo: A visual language model for few-shot
bilities in agentic systems.
                                                                              learning. Advances in Neural Information Processing
While we argue that there are viable paths for emergent                       Systems, 35:23716–23736, 2022.
capabilities, we are not saying these are automatic; we must
design/facilitate the properties that these mechanisms rely                 Anthropic. Computer use (beta) - build with Claude, 2024.
on. We must understand the mechanisms of emergence in                         URL        https://docs.anthropic.com/en/
order to intentionally design such properties into agentic                    docs/build-with-claude/computer-use.
systems and to limit the associated risk. Additionally, to                   Accessed: 2024-02-09.
reiterate a previous point, we are not advocating for the un-
constrained development of increasingly agentic systems.9                   Anthropic.  Claude code overview, 2025. URL
Rather, we argue that understanding these emergent capa-                      https://docs.anthropic.com/en/
bilities provides the AI community with essential tools for                   docs/agents-and-tools/claude-code/
mitigating their risk.10 We believe that the systems-level                    overview.
view can lead to identification of many more mechanisms
                                                                            Ashby, W. R. An Introduction to Cybernetics. Chapman &
not discussed in our paper.
                                                                              Hall, London, 1956.
These considerations will become increasingly important as
advancements in AI continue to progress. The discussion                     Åström, K. J. and Murray, R. M. Feedback Systems: An
in our present paper was largely restricted to current-day                     Introduction for Scientists and Engineers. Princeton Uni-
LLMs/agents that interact with the world via text. We as                       versity Press, Princeton, NJ, 2008.
a community need to consciously consider the impact of
additional modalities, e.g., speech, vision, touch/movement                 Bahrami, B., Olsen, K., Latham, P. E., Roepstorff, A., Rees,
(via a robotic “body”), on the overall cognitive abilities of                 G., and Frith, C. D. Optimally interacting minds. Science,
the system. Such considerations will help to ensure that AI                   329(5995):1081–1085, 2010.
safely and effectively augments human capabilities while
                                                                            Baker, G., Gibbons, R., and Murphy, K. J. Relational con-
preserving human agency.
                                                                              tracts and the theory of the firm. The Quarterly Journal
   8
      This definition of agency contributes to the growing body               of Economics, 117(1):39–84, 2002.
of literature on AI agency (Kenton et al., 2023; Barandiaran &
Almendros, 2024; Abel et al., 2025; Zhang & Varshney, 2025).                Balabanov, O. and Linander, H. Uncertainty quantifica-
    9
      See (Mitchell et al., 2025b) for a similar view.                        tion in fine-tuned LLMs using LoRA ensembles. arXiv
   10
      See (Kolt et al., 2025) for discussion of some prominent risks.         preprint arXiv:2402.12264, 2024.

                                                                        9
                                                  Agentic Systems Theory

Ballard, D. H., Hayhoe, M. M., Pook, P. K., and Rao, R. P.           Bushnell, E. W. A dual-processing approach to cross-modal
  Deictic codes for the embodiment of cognition. Behav-                matching: Implications for development. In The Develop-
  ioral and Brain Sciences, 20(4):723–742, 1997.                       ment of Intersensory Perception, pp. 19–38. Psychology
                                                                       Press, 2013.
Balleine, B. W. and Dickinson, A. Goal-directed instru-
  mental action: Contingency and incentive learning and              Cangelosi, A. and Schlesinger, M. Developmental Robotics:
  their cortical substrates. Neuropharmacology, 37(4-5):               From Babies to Robots. MIT Press, 2015.
  407–419, 1998.                                                     Chalmers, D. J. The Conscious Mind: In Search of a Fun-
                                                                       damental Theory. Oxford Paperbacks, 1997.
Bandura, A. Self-efficacy mechanism in human agency.
  American Psychologist, 37(2):122, 1982.                            Chan, A., Salganik, R., Markelius, A., Pang, C., Rajkumar,
                                                                       N., Krasheninnikov, D., Langosco, L., He, Z., Duan, Y.,
Bandura, A. Social cognitive theory: An agentic perspective.           Carroll, M., et al. Harms from increasingly agentic algo-
  Annual Review of Psychology, 52(1):1–26, 2001.                       rithmic systems. In Proceedings of the 2023 ACM Con-
                                                                       ference on Fairness, Accountability, and Transparency,
Bang, D. and Fleming, S. M. Distinct encoding of deci-                 pp. 651–666, 2023.
  sion confidence in human medial prefrontal cortex. Pro-
  ceedings of the National Academy of Sciences, 115(23):             Chawla, C., Chatterjee, S., Gadadinni, S. S., Verma, P.,
  6082–6087, 2018.                                                     and Banerjee, S. Agentic AI: The building blocks of
                                                                       sophisticated AI business applications. Journal of AI,
Bansal, G., Vaughan, J. W., Amershi, S., Horvitz, E., Four-            Robotics & Workplace Automation, 3(3):1–15, 2024.
  ney, A., Mozannar, H., Dibia, V., and Weld, D. S. Chal-
  lenges in human-agent communication. arXiv preprint                Clark, A. Whatever next? predictive brains, situated agents,
  arXiv:2412.10380, 2024.                                              and the future of cognitive science. Behavioral and Brain
                                                                       Sciences, 36(3):181–204, 2013.
Barandiaran, X. E. and Almendros, L. S. Transforming
                                                                     Da Costa, L., Tenka, S., Zhao, D., and Sajid, N. Ac-
  agency: On the mode of existence of large language
                                                                       tive inference as a model of agency. arXiv preprint
  models. arXiv preprint arXiv:2407.10735, 2024.
                                                                       arXiv:2401.12917, 2024.
Barandiaran, X. E., Di Paolo, E., and Rohde, M. Defin-               Davidson, D. and Pollock, F. Essays on actions and events.
  ing agency: Individuality, normativity, asymmetry, and               2001.
  spatio-temporality in action. Adaptive Behavior, 17(5):
  367–386, 2009.                                                     Dennett, D. C. The Intentional Stance. MIT press, 1989.

Barsalou, L. Perceptual symbol systems. The Behavioral               Denning, S. In the digital age, the combination of technol-
  and Brain Sciences, 1999.                                            ogy and radical management practices drive competitive
                                                                       advantage. Strategy & Leadership, 50(2):9–14, 2022.
Barto, A. G. Intrinsic motivation and reinforcement learning.        Di Paolo, E. A. Autopoiesis, adaptivity, teleology, agency.
  Intrinsically Motivated Learning in Natural and Artificial           Phenomenology and the Cognitive Sciences, 4(4):429–
  Systems, pp. 17–47, 2013.                                            452, 2005.
Bellman, R. Dynamic Programming. Princeton University                Dolan, R. J. and Dayan, P. Goals and habits in the brain.
  Press, 1957.                                                        Neuron, 80(2):312–325, 2013.

Boulding, K. E. The Organizational Revolution: A Study in            Edelman, G. M. Neural Darwinism. Basic Books, New
  the Ethics of Economic Organization. Harper & Brothers,              York, 1987.
  New York, 1953.
                                                                     Ellis, G. F. Top-down causation and emergence: Some
Bratman, M. Intention, Plans, and Practical Reason. Har-               comments on mechanisms. Interface Focus, 2(1):126–
  vard University Press, Cambridge, MA, 1987.                          140, 2012.
                                                                     Emirbayer, M. and Mische, A. What is agency? American
Burton, J. W., Lopez-Lopez, E., Hechtlinger, S., Rah-                 Journal of Sociology, 103(4):962–1023, 1998.
  wan, Z., Aeschbach, S., Bakker, M. A., Becker, J. A.,
  Berditchevskaia, A., Berger, J., Brinkmann, L., et al. How         Fields, C. and Levin, M. Competency in navigating arbitrary
  large language models can reshape collective intelligence.           spaces as an invariant for analyzing cognition in diverse
  Nature Human Behaviour, 8(9):1643–1655, 2024.                        embodiments. Entropy, 24(6):819, 2022.

                                                                10
                                                     Agentic Systems Theory

Fleming, S. M. and Daw, N. D. Self-evaluation of decision-              Grossman, S. J. and Hart, O. D. The costs and benefits of
  making: A general Bayesian framework for metacog-                       ownership: A theory of vertical and lateral integration.
  nitive computation. Psychological Review, 124(1):91,                    Journal of Political Economy, 94(4):691–719, 1986.
  2017.
                                                                        Hart, O. Firms, Contracts, and Financial Structure. Claren-
Franklin, S. and Graesser, A. Is it an agent, or just a pro-              don Press, 1995.
  gram? A taxonomy for autonomous agents. In Interna-
  tional Workshop on Agent Theories, Architectures, and                 Hart, O. and Moore, J. Property rights and the nature of
  Languages, pp. 21–35. Springer, 1996.                                   the firm. Journal of Political Economy, 98(6):1119–1158,
                                                                         1990.
Friston, K. Learning and inference in the brain. Neural
  Networks, 16(9):1325–1352, 2003.                                      Hauskrecht, M. Value-function approximations for partially
                                                                          observable Markov decision processes. Journal of Artifi-
Friston, K. The free-energy principle: a unified brain theory?            cial Intelligence Research, 13:33–94, 2000.
  Nature Reviews Neuroscience, 11(2):127–138, 2010.
                                                                        Hoel, E. P., Albantakis, L., and Tononi, G. Quantifying
Friston, K. J. and Stephan, K. E. Free-energy and the brain.
                                                                          causal emergence shows that macro can beat micro. Pro-
  Synthese, 159:417–458, 2007.
                                                                          ceedings of the National Academy of Sciences, 110(49):
Friston, K. J., Daunizeau, J., Kilner, J., and Kiebel, S. J. Ac-         19790–19795, 2013.
  tion and behavior: a free-energy formulation. Biological
  Cybernetics, 102:227–260, 2010.                                       Hohwy, J. The Predictive Mind. Oxford University Press,
                                                                          2013.
Frith, C. D. and Frith, U. Mechanisms of social cognition.
  Annual Review of Psychology, 63(1):287–313, 2012.                     Holland, J. H. Complex adaptive systems. Daedalus, 121
                                                                         (1):17–30, 1992.
Giddens, A. The Constitution of Society: Outline of the
  Theory of Structuration. University of California Press,              Hu, S., Lu, C., and Clune, J. Automated design of agentic
  1984.                                                                   systems. arXiv preprint arXiv:2408.08435, 2024.

Gollwitzer, P. M. Implementation intentions: Strong effects             Huang, J., Yong, S., Ma, X., Linghu, X., Li, P., Wang, Y., Li,
  of simple plans. American Psychologist, 54(7):493, 1999.                Q., Zhu, S.-C., Jia, B., and Huang, S. An embodied gener-
                                                                          alist agent in 3D world. arXiv preprint arXiv:2311.12871,
Google.  Project Mariner, 2025. URL https:                                2023a.
 //deepmind.google/technologies/
  project-mariner/.                                                     Huang, K., Qu, Y., Cousins, H., Johnson, W. A., Yin, D.,
                                                                          Shah, M., Zhou, D., Altman, R., Wang, M., and Cong, L.
Gopnik, A. Childhood as a solution to explore–exploit
                                                                          Crispr-GPT: An LLM agent for automated design of gene-
  tensions. Philosophical Transactions of the Royal Society
                                                                          editing experiments. arXiv preprint arXiv:2404.18021,
 B, 375(1803):20190502, 2020.
                                                                          2024a.
Gopnik, A. and Wellman, H. M. Reconstructing construc-
  tivism: Causal models, bayesian learning mechanisms,                  Huang, Q., Wake, N., Sarkar, B., Durante, Z., Gong, R.,
  and the theory theory. Psychological Bulletin, 138(6):                 Taori, R., Noda, Y., Terzopoulos, D., Kuno, N., Famoti,
 1085, 2012.                                                             A., et al. Position paper: Agent AI towards a holistic
                                                                          intelligence. arXiv preprint arXiv:2403.00833, 2024b.
Gopnik, A., Meltzoff, A. N., and Kuhl, P. K. The Scientist
  in the Crib: Minds, Brains, and How Children Learn.                   Huang, S., Dong, L., Wang, W., Hao, Y., Singhal, S., Ma,
 William Morrow & Co, 1999.                                               S., Lv, T., Cui, L., Mohammed, O. K., Patra, B., et al.
                                                                          Language is not all you need: Aligning perception with
Greenblatt, R., Denison, C., Wright, B., Roger, F., MacDi-                language models. Advances in Neural Information Pro-
  armid, M., Marks, S., Treutlein, J., Belonax, T., Chen, J.,             cessing Systems, 36:72096–72109, 2023b.
  Duvenaud, D., et al. Alignment faking in large language
  models. arXiv preprint arXiv:2412.14093, 2024.                        Hutchins, E. Cognition in the Wild. MIT press, 1995.

Griot, M., Hemptinne, C., Vanderdonckt, J., and Yuksel, D.              Jin, Z., Liu, J., Lyu, Z., Poff, S., Sachan, M., Mihalcea,
  Large language models lack essential metacognition for                   R., Diab, M., and Schölkopf, B. Can large language
  reliable medical reasoning. Nature Communications, 16                    models infer causation from correlation? arXiv preprint
  (1):642, 2025.                                                           arXiv:2306.05836, 2023.

                                                                   11
                                                  Agentic Systems Theory

Johnson, S. G., Karimi, A.-H., Bengio, Y., Chater, N., Ger-          Lin, Z., Trivedi, S., and Sun, J. Generating with confidence:
  stenberg, T., Larson, K., Levine, S., Mitchell, M., Rah-             Uncertainty quantification for black-box large language
  wan, I., Schölkopf, B., et al. Imagining and building wise          models. arXiv preprint arXiv:2305.19187, 2023.
  machines: The centrality of AI metacognition. arXiv
  preprint arXiv:2411.02478, 2024.                                   Lu, C., Lu, C., Lange, R. T., Foerster, J., Clune, J., and Ha,
                                                                       D. The AI scientist: Towards fully automated open-ended
Kapkiç, A., Mandal, P., Wan, S., Sheth, P., Gorantla, A.,             scientific discovery. arXiv preprint arXiv:2408.06292,
  Choi, Y., Liu, H., and Candan, K. S. Introducing causal-             2024.
  bench: A flexible benchmark framework for causal analy-
  sis and machine learning. In Proceedings of the 33rd ACM           Maes, P. Modeling adaptive autonomous agents. Artificial
  International Conference on Information and Knowledge               Life, 1(1 2):135–162, 1993.
 Management, pp. 5220–5224, 2024.                                    Meinke, A., Schoen, B., Scheurer, J., Balesni, M., Shah,
                                                                      R., and Hobbhahn, M. Frontier models are capable of
Kenton, Z., Kumar, R., Farquhar, S., Richens, J., MacDer-
                                                                      in-context scheming. arXiv preprint arXiv:2412.04984,
  mott, M., and Everitt, T. Discovering agents. Artificial
                                                                      2024.
  Intelligence, 322:103963, 2023.
                                                                     Merleau-Ponty, M. Phénoménologie de la perception. Gal-
Kolt, N. Governing AI agents. Available at SSRN, 2024.
                                                                      limard, Paris, 1945.
Kolt, N., Shur-Ofry, M., and Cohen, R. Lessons from
                                                                     Mitchell, M., Ghosh, A., Luccioni, A. S., and Pistilli, G.
  complexity theory for AI governance. arXiv preprint
                                                                      AI agents are here. What now?, 1 2025a. URL https:
  arXiv:2502.00012, 2025.
                                                                      //huggingface.co/blog/ethics-soc-7.
Kuhn, T. S. The Structure of Scientific Revolutions. Univer-
                                                                     Mitchell, M., Ghosh, A., Luccioni, A. S., and Pistilli, G.
  sity of Chicago Press, 1997.
                                                                      Fully autonomous AI agents should not be developed.
Kumar, P. R. and Varaiya, P. Stochastic Systems: Estimation,          arXiv preprint arXiv:2502.02649, 2025b.
  Identification, and Adaptive Control. Prentice-Hall, 1986.
                                                                     Mitelut, C., Smith, B., and Vamplew, P. Position: Intent-
Lai, C.-H., Lin, S. H., et al. Systems theory. The Interna-           aligned AI systems must optimize for agency preserva-
  tional Encyclopedia of Organizational Communication,                tion. In Forty-first International Conference on Machine
  41:1–18, 2017.                                                      Learning, 2024.

Lake, B. M., Ullman, T. D., Tenenbaum, J. B., and Gersh-             Narayanan, S., Braza, J. D., Griffiths, R.-R., Ponnapati, M.,
  man, S. J. Building machines that learn and think like               Bou, A., Laurent, J., Kabeli, O., Wellawatte, G., Cox,
  people. Behavioral and Brain Sciences, 40:e253, 2017.                S., Rodriques, S. G., et al. Aviary: Training language
                                                                       agents on challenging scientific tasks. arXiv preprint
Levin, M. The computational boundary of a “self”: develop-             arXiv:2412.21154, 2024.
  ment, cancer and the continuum of embryonic regulatory
  networks. Philosophical Transactions of the Royal Soci-            Nelson, T. O. Consciousness and metacognition. American
  ety B: Biological Sciences, 374(1774):20180376, 2019.                Psychologist, 51(2):102, 1996.

Li, K., Wang, J., Yang, L., Lu, C., and Dai, B. Sem-                 Niu, N., Wu, Y., Li, H., Li, M., Yang, D., Fan, W., and
  Grasp: Semantic grasp generation via language aligned                Zhong, Y. Influence of voluntary action and outcome
  discretization. In European Conference on Computer                   valence on the sense of agency. Frontiers in Human
  Vision, pp. 109–127. Springer, 2025.                                 Neuroscience, 17:1206858, 2023.

Li, X., Wang, S., Zeng, S., Wu, Y., and Yang, Y. A survey            OpenAI.            Introducing Operator,          1 2025.
  on LLM-based multi-agent systems: Workflow, infras-                  URL                https://openai.com/index/
  tructure, and challenges. Vicinagearth, 1(1):9, 2024a.               introducing-operator/.                 A research pre-
                                                                      view of an agent that can use its own browser to perform
Li, Y., Huang, Y., Lin, Y., Wu, S., Wan, Y., and Sun, L.               tasks for you. Available to Pro users in the U.S.
  I think, therefore I am: Awareness in large language
  models. arXiv preprint arXiv:2401.17882, 2024b.                    OpenAI, Andrychowicz, M., Baker, B., Chociej, M.,
                                                                      Józefowicz, R., McGrew, B., Pachocki, J., Petron, A.,
Lieder, F. and Griffiths, T. L. Resource-rational analysis:            Plappert, M., Powell, G., Ray, A., Schneider, J., Sidor,
  Understanding human cognition as the optimal use of                  S., Tobin, J., Welinder, P., Weng, L., and Zaremba, W.
  limited computational resources. Behavioral and Brain                Learning dexterous in-hand manipulation. CoRR, 2018.
  Sciences, 43:1–60, 2020.                                             URL http://arxiv.org/abs/1808.00177.

                                                                12
                                                    Agentic Systems Theory

Oudeyer, P.-Y., Kaplan, F., and Hafner, V. V. Intrinsic                Shavit, Y., Agarwal, S., Brundage, M., Adler, S., O’Keefe,
  motivation systems for autonomous mental development.                  C., Campbell, R., Lee, T., Mishkin, P., Eloundou, T.,
 IEEE Transactions on Evolutionary Computation, 11(2):                   Hickey, A., et al. Practices for governing agentic AI
  265–286, 2007.                                                         systems. Research Paper, OpenAI, December, 2023.

Pearl, J. Causality. Cambridge university press, 2009.                 Shea, N., Boldt, A., Bang, D., Yeung, N., Heyes, C.,
                                                                         and Frith, C. D. Supra-personal cognitive control and
Pedreschi, D., Pappalardo, L., Ferragina, E., Baeza-Yates,               metacognition. Trends in Cognitive Sciences, 18(4):186–
  R., Barabási, A.-L., Dignum, F., Dignum, V., Eliassi-Rad,             193, 2014.
  T., Giannotti, F., Kertész, J., et al. Human-AI coevolution.
  Artificial Intelligence, pp. 104244, 2024.                           Shen, H., Knearem, T., Ghosh, R., Alkiek, K., Krishna, K.,
                                                                         Liu, Y., Ma, Z., Petridis, S., Peng, Y.-H., Qiwei, L., et al.
Piaget, J. The Origins of Intelligence in Children. Interna-             Towards bidirectional human-AI alignment: A systematic
  tional University Press, Inc., New York, 1952.                         review for clarifications, framework, and future directions.
                                                                         arXiv preprint arXiv:2406.09264, 2024.
Qiu, J., Lam, K., Li, G., Acharya, A., Wong, T. Y., Darzi, A.,
  Yuan, W., and Topol, E. J. LLM-based agentic systems                 Shorinwa, O., Mei, Z., Lidard, J., Ren, A. Z., and Majum-
  in medicine and healthcare. Nature Machine Intelligence,               dar, A. A survey on uncertainty quantification of large
  6(12):1418–1420, 2024.                                                 language models: Taxonomy, open research challenges,
                                                                         and future directions. arXiv preprint arXiv:2412.05563,
Rao, A., Khandelwal, A., Tanmay, K., Agarwal, U., and
                                                                         2024.
  Choudhury, M. Ethical reasoning over moral alignment:
  A case and framework for in-context ethical policies in
                                                                       Silver, D., Huang, A., Maddison, C. J., Guez, A., Sifre, L.,
  LLMs. arXiv preprint arXiv:2310.07251, 2023.
                                                                         Van Den Driessche, G., Schrittwieser, J., Antonoglou, I.,
Reed, S., Zolna, K., Parisotto, E., Colmenarejo, S. G.,                   Panneershelvam, V., Lanctot, M., et al. Mastering the
  Novikov, A., Barth-Maron, G., Gimenez, M., Sulsky,                      game of Go with deep neural networks and tree search.
  Y., Kay, J., Springenberg, J. T., et al. A generalist agent.            nature, 529(7587):484–489, 2016.
  arXiv preprint arXiv:2205.06175, 2022.
                                                                       Silver, D., Schrittwieser, J., Simonyan, K., Antonoglou,
Richens, J. and Everitt, T. Robust agents learn causal world              I., Huang, A., Guez, A., Hubert, T., Baker, L., Lai, M.,
  models. arXiv preprint arXiv:2402.10877, 2024.                          Bolton, A., et al. Mastering the game of Go without
                                                                          human knowledge. nature, 550(7676):354–359, 2017.
Romanou, A., Montariol, S., Paul, D., Laugier, L., Aberer,
  K., and Bosselut, A. CRAB: Assessing the strength of                 Simon, H. A. Models of Man: Social and Rational - Mathe-
  causal relationships between real-world events. arXiv                  matical Essays on Rational Human Behavior in a Social
  preprint arXiv:2311.04284, 2023.                                       Setting. John Wiley and Sons, New York, 1957.

Rosenblueth, A., Wiener, N., and Bigelow, J. Behavior,                 Simon, H. A. The architecture of complexity. In The Roots
  purpose and teleology. Philosophy of Science, 10(1):                   of Logistics, pp. 335–361. Springer, 2012.
 18–24, 1943.
                                                                       Singh, S., Lewis, R. L., Barto, A. G., and Sorg, J. Intrinsi-
Rouleau, N. and Levin, M. Discussions of machine versus                  cally motivated reinforcement learning: An evolutionary
  living intelligence need more clarity. Nature Machine                  perspective. IEEE Transactions on Autonomous Mental
  Intelligence, 6(12):1424–1426, 2024.                                   Development, 2(2):70–82, 2010.

Russell, S. J. and Norvig, P. Artificial Intelligence:                 Singh, S. P., Jaakkola, T., and Jordan, M. I. Learning with-
  A Modern Approach. Prentice Hall, 1995. ISBN                           out state-estimation in partially observable Markovian
  9780131038059.                                                         decision processes. In Machine Learning Proceedings
                                                                         1994, pp. 284–292. Elsevier, 1994.
Schlosser, M. Agency. https://plato.stanford.
  edu/entries/agency/, October 2019. Accessed:                         Smith, L. and Gasser, M. The development of embodied
  2025-01-06.                                                            cognition: Six lessons from babies. Artificial Life, 11
                                                                         (1-2):13–29, 2005.
Scholten, F., Rebholz, T. R., and Hütter, M. Metacogni-
  tive myopia in large language models. arXiv preprint                 Spelke, E. S. and Kinzler, K. D. Core knowledge. Develop-
  arXiv:2408.05568, 2024.                                                mental Science, 10(1):89–96, 2007.

                                                                  13
                                                 Agentic Systems Theory

Sun, Q., Yu, Q., Cui, Y., Zhang, F., Zhang, X., Wang,              Von Wright, G. H. Explanation and Understanding. Cornell
  Y., Gao, H., Liu, J., Huang, T., and Wang, X. Gen-                 University Press, 2004.
  erative pretraining in multimodality. arXiv preprint
  arXiv:2307.05222, 2023.                                          Wegner, D. Transactive memory: A contemporary analysis
                                                                    of the group mind. Theories of group behavior/Springer-
Surowiecki, J. The Wisdom of Crowds: Why the Many                   Verlag, 1987.
  Are Smarter Than the Few and How Collective Wisdom
  Shapes Business, Economies, Societies and Nations. Dou-          Wiener, N. Cybernetics: Or Control and Communication
  bleday, 2004.                                                     in the Animal and the Machine. MIT Press, Cambridge,
                                                                    MA, 1948.
Sutton, R. S. and Barto, A. G. Reinforcement Learning: An
  Introduction. MIT Press, 1998.                                   Wiesinger, J., Marlow, P., and Vuskovic, V.           Agents.
                                                                    Whitepaper, Google, 2025.
Tavafoghi, H., Ouyang, Y., and Teneketzis, D. A suffi-
  cient information approach to decentralized decision mak-        Williamson, O. E. Markets and Hierarchies: Analysis and
  ing. In 2018 IEEE Conference on Decision and Control              Antitrust Implications. Free Press, New York, 1975.
  (CDC), pp. 5069–5076. IEEE, 2018.
                                                                   Wolf, W. and Tomasello, M. A shared intentionality ac-
Tavafoghi, H., Ouyang, Y., and Teneketzis, D. A unified             count of uniquely human social bonding. Perspectives on
  approach to dynamic decision problems with asymmetric             Psychological Science, pp. 17456916231201795, 2023.
  information: Nonstrategic agents. IEEE Transactions on
  Automatic Control, 67(3):1105–1119, 2021.                        Xu, D. and Rivera, J.-P.        Towards measuring
                                                                     goal-directedness in AI systems.  arXiv preprint
Tenenbaum, J. B., Kemp, C., Griffiths, T. L., and Goodman,           arXiv:2410.04683, 2024.
  N. D. How to grow a mind: Statistics, structure, and
  abstraction. Science, 331(6022):1279–1285, 2011.                 Xu, F. F., Song, Y., Li, B., Tang, Y., Jain, K., Bao, M., Wang,
                                                                     Z. Z., Zhou, X., Guo, Z., Cao, M., et al. TheAgentCom-
Thompson, E. Living ways of sense making. Philosophy                 pany: Benchmarking LLM agents on consequential real
  Today, 55(Supplement):114–123, 2011.                               world tasks. arXiv preprint arXiv:2412.14161, 2024.
Thompson, V. A., Turner, J. A. P., and Pennycook, G. Intu-         Xu, Z., Qi, B., Agrawal, S., and Song, S. AdaGrasp: Learn-
  ition, reason, and metacognition. Cognitive Psychology,            ing an adaptive gripper-aware grasping policy. In 2021
  63(3):107–140, 2011.                                               IEEE International Conference on Robotics and Automa-
Thorndike, E. L. A constant error in psychological ratings.          tion (ICRA), pp. 4620–4626. IEEE, 2021.
  Journal of Applied Psychology, 4(1):25–29, 1920.                 Yang, L., Shirvaikar, V., Clivio, O., and Falck, F. A critical
Tran, K.-T., Dao, D., Nguyen, M.-D., Pham, Q.-V.,                    review of causal reasoning benchmarks for large language
  O’Sullivan, B., and Nguyen, H. D. Multi-agent collabo-             models. In AAAI 2024 Workshop on”Are Large Language
  ration mechanisms: A survey of LLMs. arXiv preprint                Models Simply Causal Parrots?”, 2024.
  arXiv:2501.06322, 2025.                                          Yeung, N., Botvinick, M. M., and Cohen, J. D. The neural
Vafa, K., Chen, J. Y., Kleinberg, J., Mullainathan, S., and          basis of error detection: Conflict monitoring and the error-
  Rambachan, A. Evaluating the world model implicit in               related negativity. Psychological Review, 111(4):931,
  a generative model. arXiv preprint arXiv:2406.03689,               2004.
  2024.
                                                                   Zečević, M., Willig, M., Dhami, D. S., and Kersting, K.
Varela, F. J. Living ways of sense-making: A middle path             Causal parrots: Large language models may talk causality
  for neuroscience. In Order and Disorder: Proceedings               but are not causal. arXiv preprint arXiv:2308.13067,
  of the Stanford International Symposium, pp. 208–224.              2023.
  Anma Libri, 1984.
                                                                   Zhang, A. and Varshney, L. R. Conceptualizing agency: A
Varela, F. J., Thompson, E., and Rosch, E. The Embodied              framework for human-AI interaction. In HAI-GEN 2025:
  Mind: Cognitive Science and Human Experience. MIT                  6th Workshop on Human-AI Co-Creation with Generative
  Press, Cambridge, MA, 1991.                                        Models, 2025.

Von Bertalanffy, L. General System Theory: Foundations,            Zhang, D., Yu, Y., Dong, J., Li, C., Su, D., Chu, C., and Yu,
  Development, Applications. George Braziller, New York,             D. MM-LLMs: Recent advances in multimodal large lan-
  1968.                                                              guage models. arXiv preprint arXiv:2401.13601, 2024.

                                                              14
                                                  Agentic Systems Theory

Zhou, Y., Wu, X., Huang, B., Wu, J., Feng, L., and Tan,
  K. C. Causalbench: A comprehensive benchmark for
  causal learning capability of large language models. arXiv
  preprint arXiv:2404.06349, 2024.
Zhu, A., Dugan, L., and Callison-Burch, C. ReDel: A
  toolkit for LLM-powered recursive multi-agent systems.
  arXiv preprint arXiv:2408.02248, 2024.




                                                               15
