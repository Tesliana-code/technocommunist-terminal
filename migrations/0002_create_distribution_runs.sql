CREATE TABLE distribution_runs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,

    source_draft_id TEXT NOT NULL
        REFERENCES distribution_drafts(id)
        ON DELETE RESTRICT,

    source_revision BIGINT NOT NULL
        CHECK (source_revision >= 0),

    draft_snapshot JSONB NOT NULL,
    preview_snapshot JSONB NOT NULL,

    protocol_version TEXT NOT NULL
        CHECK (btrim(protocol_version) <> ''),

    engine_version TEXT NOT NULL
        CHECK (btrim(engine_version) <> ''),

    manual_version TEXT NOT NULL
        CHECK (btrim(manual_version) <> ''),

    finalized_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT distribution_runs_unique_draft_revision
        UNIQUE (source_draft_id, source_revision),

    CONSTRAINT distribution_runs_draft_snapshot_is_object
        CHECK (jsonb_typeof(draft_snapshot) = 'object'),

    CONSTRAINT distribution_runs_preview_snapshot_is_object
        CHECK (jsonb_typeof(preview_snapshot) = 'object')
);

CREATE FUNCTION reject_distribution_run_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'distribution_runs are immutable';
END;
$$;

CREATE TRIGGER distribution_runs_reject_update
BEFORE UPDATE ON distribution_runs
FOR EACH ROW
EXECUTE FUNCTION reject_distribution_run_mutation();

CREATE TRIGGER distribution_runs_reject_delete
BEFORE DELETE ON distribution_runs
FOR EACH ROW
EXECUTE FUNCTION reject_distribution_run_mutation();
