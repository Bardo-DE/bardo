use computer_use_geometry::{GridMax, ModelSpace};
use computer_use_session::{
    ComputerUseProfile, ComputerUseSession, ComputerUseTaskText, FrameBudget, RepairBudget,
};
use model_types::ModelName;
use model_types::{ActionProfile, ComputerUseDialect, ToolDialect};
use model_vision_prep::{Encoding, PatchFactor, PixelCount, ResizeRule};

fn profile() -> ComputerUseProfile {
    ComputerUseProfile {
        dialect: ComputerUseDialect::Tool(ToolDialect::Holo31),
        actions: ActionProfile::default(),
        rule: ResizeRule::SmartResize {
            factor: PatchFactor(32),
            min_pixels: PixelCount(65_536),
            max_pixels: PixelCount(16_777_216),
        },
        space: ModelSpace::Grid(GridMax(1000)),
        history: FrameBudget(3),
        repair: RepairBudget(1),
        encoding: Encoding::Png,
    }
}

#[test]
fn begin_builds_a_session_without_touching_the_model() {
    let task = ComputerUseTaskText {
        goal: "save the file".into(),
        hints: vec![],
    };
    let _session = ComputerUseSession::begin(profile(), task.clone(), ModelName("holo".into()));
    assert_eq!(
        format!("{task:?}"),
        "ComputerUseTaskText(<goal 13 chars, 0 hints>)"
    );
}
