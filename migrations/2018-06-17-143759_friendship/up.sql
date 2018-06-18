
create table friendship (
    source text not null references users (username),
    target text not null references users (username),
    accepted_at timestamptz,
    check (source <> target),
    primary key (source, target) -- cannot create primary key from index created below.
);

create unique index idx_fship on friendship (
    greatest(source, target),
    least(source, target)
);

-- alter table friendship add constraint friendship_pkey primary key using index idx_fship;
