Document contains all manual data migrations from the historic AIS api to our live postgres database in azure.
Each entry should start with a heading with the date the migration was done followed by the start/end thresholds used, and optionally a reason
to indicate why it was done.
This document exists incase we find some "holes" in our AIS data that might overlap with migration dates.

# 2026-10-28

start_threshold: 2026-10-27T14:40:00.000+0100
end_threshold: 2026-10-28T08:10:00.000+0100

Reason: Stopped ais-consumer due to pg_dump.

# 2026-10-26

start_threshold: 2026-10-26T13:00:00.000+0100
end_threshold: 2026-10-26T17:10:00.000+0100

Reason: Upgraded Postgres from 17 to 18.

# 2026-09-30

start_threshold: 2026-09-21T00:00:00+0200
end_threshold: 2026-09-30T12:25:00+0200

Reason: Lost extended AIS for about 10 days.
