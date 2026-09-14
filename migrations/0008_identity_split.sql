ALTER TABLE identity_audit DROP CONSTRAINT identity_audit_kind_check;
ALTER TABLE identity_audit ADD CONSTRAINT identity_audit_kind_check
    CHECK (kind IN ('linked', 'merged', 'split'));
