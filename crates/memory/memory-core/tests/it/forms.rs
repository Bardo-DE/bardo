//! The stored forms the timeline reads.

use memory_core::*;

#[test]
fn the_timeline_has_a_bucket_for_every_kind_of_actor() {
    let slugs: Vec<&str> = ActorFilter::ALL.iter().map(|f| f.slug()).collect();
    assert_eq!(
        slugs,
        [
            "everyone",
            "you",
            "companion",
            "terminal",
            "mcp",
            "apps",
            "unknown"
        ]
    );
}
