CREATE TABLE
    user_follows (
        barentswatch_user_id UUID,
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        PRIMARY KEY (barentswatch_user_id, fiskeridir_vessel_id)
    );
