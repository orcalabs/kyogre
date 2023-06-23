use crate::helper::*;
use engine::*;
use orca_statemachine::Schedule;

#[tokio::test]
async fn test_pending_restarts_interrupted_scrape_chain_on_restart_during_scrape_state() {
    let config = Config {
        scrape_schedule: Schedule::Periodic(std::time::Duration::from_micros(0)),
    };
    test(config, |helper| async move {
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
    })
    .await;
}

#[tokio::test]
async fn test_pending_enters_sleep_state_if_no_states_are_ready() {
    let config = Config {
        scrape_schedule: Schedule::Disabled,
    };
    test(config, |helper| async move {
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Sleep
        );
    })
    .await;
}

#[tokio::test]
async fn test_restarts_scrape_chain_when_prior_destination_state_was_pending() {
    let config = Config {
        scrape_schedule: Schedule::Periodic(std::time::Duration::from_micros(0)),
    };
    test(config, |mut helper| async move {
        assert_eq!(
            helper.run_step(EngineDiscriminants::Sleep).await,
            EngineDiscriminants::Pending
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
        helper.disable_scrape();
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
    })
    .await;
}

#[tokio::test]
async fn test_restarts_scrape_chain_at_last_state_if_interrupted() {
    let config = Config {
        scrape_schedule: Schedule::Periodic(std::time::Duration::from_micros(0)),
    };
    test(config, |helper| async move {
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Scrape).await,
            EngineDiscriminants::Trips
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Trips
        );
    })
    .await;
}

#[tokio::test]
async fn test_does_not_restart_scrape_if_chain_was_completed_but_was_interrupted_earlier() {
    let config = Config {
        scrape_schedule: Schedule::Periodic(std::time::Duration::from_micros(0)),
    };

    test(config, |mut helper| async move {
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Scrape
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Scrape).await,
            EngineDiscriminants::Trips
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Trips).await,
            EngineDiscriminants::TripsPrecision
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::TripsPrecision).await,
            EngineDiscriminants::Benchmark
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Benchmark).await,
            EngineDiscriminants::HaulDistribution
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::HaulDistribution).await,
            EngineDiscriminants::TripDistance
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::TripDistance).await,
            EngineDiscriminants::Pending
        );
        helper.disable_scrape();
        assert_eq!(
            helper.run_step(EngineDiscriminants::TripDistance).await,
            EngineDiscriminants::Pending
        );
        assert_eq!(
            helper.run_step(EngineDiscriminants::Pending).await,
            EngineDiscriminants::Sleep
        );
    })
    .await;
}
