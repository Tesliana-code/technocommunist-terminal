CREATE TABLE distribution_drafts (
    id TEXT PRIMARY KEY
        CHECK (btrim(id) <> ''),

    revision BIGINT NOT NULL DEFAULT 0
        CHECK (revision >= 0),

    body JSONB NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT distribution_drafts_body_is_object
        CHECK (jsonb_typeof(body) = 'object'),

    CONSTRAINT distribution_drafts_body_excludes_identity
        CHECK (
            NOT (body ? 'id')
            AND NOT (body ? 'revision')
        )
);
