//! Compaction in the driver: before a planner turn, a task that has outgrown its budget has its
//! oldest whole steps summarised once, and the summary stands in for them from then on. The choice
//! of what to cover and what the summariser may see are `agent-loop`'s; the model turn is the
//! planner's (`Planner::summarise`). The task keeps every step it ever had: the summary is
//! only what the assembler shows in their place.
//!
//! A summary that cannot be made (the model is away, was cancelled, or wrote something the checks
//! refuse) leaves the task as it was, and the assembler falls back to dropping the oldest masked
//! steps.

use crate::deadline::bounded;
use crate::runtime::Companion;
use crate::seams::Surface;
use action_client::Transport as IntentsTransport;
use action_core::Summary;
use action_vocab::Clock;
use agent_loop::{Sources, compaction_due, summarised, summary_input};
use agent_planner::Planner;
use provenance::TaskId;

impl<P: Planner, I: IntentsTransport, K: Clock, S: Surface> Companion<P, I, K, S> {
    /// The summary that now stands for the start of `task`, when it was over its budget and one
    /// was made; `None` when nothing was due or the attempt failed.
    pub(crate) async fn compact(&mut self, task: &TaskId, sources: &Sources) -> Option<Summary> {
        let covers = compaction_due(&self.config.assembler, sources)?;
        let input = summary_input(sources, covers);
        let asked = bounded(
            &*self.shared,
            &self.clock,
            task,
            self.waits.model,
            self.planner.summarise(&input),
        )
        .await;
        let made = match asked {
            Ok(Ok(made)) => made,
            Ok(Err(fault)) => {
                tracing::warn!(%task, %fault, "the task's summary could not be made");
                return None;
            }
            Err(_) => return None,
        };
        let summary = summarised(&input, made.text);
        if let Some(rt) = self.runtimes.get_mut(task) {
            rt.summary = Some(summary.clone());
        }
        Some(summary)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Companion, Quiet};
    use account_capability::Tier;
    use account_core::{AppId, AppName, Isolation};
    use action_client::{InProcess, Intents};
    use action_core::{
        AgentConfig, Covers, StepEnd, StepLine, StepShown, Summary, SummaryText, TurnVia, UserTurn,
    };
    use action_fake::{FixedClock, fake_router};
    use action_vocab::{ActionRef, CallId, Caller, CallerRole, LabelText, TurnId};
    use action_wire::{CallerId, SessionOpen};
    use agent_loop::{Sources, assemble, compaction_due};
    use agent_planner::{ScriptedPlanner, Summarised};
    use agent_session::{
        BackendKind, Compacted, CompactedBy, Logged, Opening, Read, Seq, SessionEntry, TaintCause,
        TaintNote, resume_plan,
    };
    use base_ids::{Count, Tokens};
    use provenance::{ActionName, AgentRef, Effect, Integrity, SpaceId, TaskId, UnixSeconds};
    use std::{collections::BTreeSet, sync::Arc};

    fn step(n: u64) -> StepLine {
        StepLine {
            call: CallId(n),
            action: ActionRef {
                app: AppName::parse("org.bardo.Shell").expect("app"),
                name: ActionName::parse("demo.item.archive").expect("action"),
            },
            effect: Effect::UndoableWrite,
            end: StepEnd::Done {
                said: Some(LabelText::parse("Archived 12 items").expect("said")),
                value: None,
                undo: None,
            },
            shown: StepShown::Full,
            with: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_task_over_its_budget_is_summarised_once_and_keeps_every_step() {
        let router = Arc::new(fake_router(AgentConfig::default()).expect("router"));
        let shell = AppName::parse("org.bardo.Shell").expect("app");
        let caller = CallerId {
            app: AppId {
                name: shell.clone(),
                isolation: Isolation::Unsandboxed,
            },
            roles: BTreeSet::from([CallerRole::Companion]),
        };
        let mut companion: Companion<_, _, FixedClock, Quiet> = Companion::new(
            Intents::over(InProcess::new(router, caller)),
            ScriptedPlanner::default().with_summaries([Ok(Summarised {
                text: SummaryText::parse("Archived the earlier items in batches.", &[])
                    .expect("a summary"),
                tier: Tier::Balanced,
                served: None,
            })]),
            AgentConfig::default(),
            FixedClock::at(UnixSeconds(1_000)),
            shell,
        );
        let opened = companion
            .open(SessionOpen {
                space: SpaceId::parse("work").expect("space"),
                agent: AgentRef::Companion,
                parent: None,
                cwd: None,
                started_from: None,
                external: None,
                helper: None,
                acts_for: None,
            })
            .await
            .expect("open");
        let task = opened.task.clone();
        companion.runtimes.get_mut(&task).expect("runtime").history = (1..=40).map(step).collect();
        companion.config.assembler.task = Tokens(700);

        let sources = companion.sources(&task).await;
        let summary = companion
            .compact(&task, &sources)
            .await
            .expect("over its budget, so summarised");
        assert!(summary.covers.steps.0 >= 1 && summary.covers.steps.0 <= 37);
        assert_eq!(
            companion.runtimes[&task].history.len(),
            40,
            "no step is lost"
        );
        assert_eq!(companion.runtimes[&task].summary, Some(summary.clone()));

        let again = companion.sources(&task).await;
        assert_eq!(again.summary, Some(summary));
        assert_eq!(compaction_due(&companion.config.assembler, &again), None);
        assert_eq!(
            companion.compact(&task, &again).await,
            None,
            "once is enough"
        );
    }

    fn shell() -> AppName {
        AppName::parse("org.bardo.Shell").expect("app")
    }

    fn opening() -> Opening {
        Opening {
            task: TaskId::parse("t-1").expect("task"),
            space: SpaceId::parse("work").expect("space"),
            opener: Some(shell()),
            agent: Some(AgentRef::Companion),
            backend: BackendKind::Native,
            parent: None,
            forked_from: None,
            cwd: None,
            started_from: None,
            label: None,
            rewind: Default::default(),
        }
    }

    fn turn() -> UserTurn {
        UserTurn {
            id: TurnId(1),
            text: "archive the old items".into(),
            at: UnixSeconds(1),
            from: Caller::launcher(),
            via: TurnVia::Typed,
            mode: None,
        }
    }

    fn sources(history: Vec<StepLine>, taint: Integrity, summary: Option<Summary>) -> Sources {
        Sources {
            cards: vec![],
            profile: vec![],
            primer: None,
            rollup: None,
            roster: action_core::Roster::default(),
            episodes: vec![],
            recalled: vec![],
            context: crate::nowhere(shell()),
            turns: vec![turn()],
            history,
            handles: vec![],
            inbox: vec![],
            taint,
            task_policy: None,
            skills: vec![],
            skill_texts: vec![],
            shown: vec![],
            summary,
        }
    }

    fn rows(entries: Vec<SessionEntry>) -> Vec<Logged> {
        entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| Logged {
                seq: Seq(i as u64),
                read: Read::Entry(Box::new(e)),
            })
            .collect()
    }

    #[test]
    fn a_restore_after_compaction_shows_the_planner_the_same_view_and_keeps_the_taint() {
        let budget = AgentConfig::default().assembler;
        let covers = Covers {
            turns: Count(0),
            steps: Count(30),
        };
        let text = SummaryText::parse("archived the first thirty items", &[]).expect("summary");
        for (written, integrity) in [(false, Integrity::Trusted), (true, Integrity::Untrusted)] {
            let mut entries = vec![SessionEntry::Opened(opening())];
            if written {
                entries.push(SessionEntry::Taint(TaintNote {
                    cause: TaintCause::UntrustedReveal,
                    at_call: None,
                }));
            }
            entries.push(SessionEntry::Turn(turn()));
            entries.extend((1..=40).map(|n| SessionEntry::Step(step(n))));
            entries.push(SessionEntry::Compacted(Compacted {
                covers,
                text: text.clone(),
                taint: written,
                by: CompactedBy {
                    tier: Tier::Balanced,
                    model: None,
                },
            }));
            let plan = resume_plan(&rows(entries)).expect("plan");
            assert!(plan.faults.is_empty(), "{written:?}: {:?}", plan.faults);
            assert_eq!(plan.history.len(), 40, "the log still holds every step");
            assert_eq!(plan.live_history().len(), 10);

            let before = sources(
                (1..=40).map(step).collect(),
                integrity,
                Some(Summary {
                    covers,
                    text: text.clone(),
                    integrity,
                }),
            );
            let after = sources(plan.history.clone(), integrity, plan.summary.clone());
            let (seen, restored) = (assemble(&budget, &before), assemble(&budget, &after));
            assert_eq!(seen, restored, "{written:?}");
            assert_eq!(restored.history.len(), 10);
            assert_eq!(restored.summary.map(|s| s.integrity), Some(integrity));
        }
    }
}
