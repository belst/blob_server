
create table if not exists users (
    username text not null primary key,
    token uuid not null unique default gen_random_uuid(),
    last_location geography(POINT) not null,
    last_online timestamptz not null default now(),
    completion integer not null default 0
);