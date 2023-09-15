use fiskeridir_rs::AquaCultureEntry;

use crate::*;

pub struct ManualDeliveryPointsBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct ManualDeliveryPointConstructor {
    pub val: ManualDeliveryPoint,
}

pub struct MattilsynetBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct MattilsynetConstructor {
    pub val: MattilsynetDeliveryPoint,
}

pub struct AquaCultureBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct AquaCultureConstructor {
    pub val: AquaCultureEntry,
}

impl AquaCultureBuilder {
    pub async fn persist(mut self) -> TestStateBuilder {
        self.state
            .storage
            .add_aqua_culture_register(self.state.aqua_cultures.drain(..).map(|v| v.val).collect())
            .await
            .unwrap();

        self.base()
    }
}
