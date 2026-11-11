-- Run once on installations created with the original personal-cover schema.
CREATE TABLE organizations (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(120) NOT NULL,
    slug VARCHAR(80) NOT NULL UNIQUE,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE users
    ADD COLUMN organization_id BIGINT UNSIGNED NULL AFTER role,
    ADD COLUMN organization_role ENUM('admin', 'member') NULL AFTER organization_id,
    ADD CONSTRAINT fk_users_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL,
    ADD INDEX idx_users_organization (organization_id, is_active);

ALTER TABLE cover_sets
    ADD COLUMN organization_id BIGINT UNSIGNED NULL AFTER user_id,
    ADD COLUMN personal_user_id BIGINT UNSIGNED GENERATED ALWAYS AS (IF(organization_id IS NULL, user_id, NULL)) STORED AFTER is_default,
    ADD COLUMN default_user_id BIGINT UNSIGNED GENERATED ALWAYS AS (IF(is_default = 1 AND organization_id IS NULL, user_id, NULL)) STORED AFTER personal_user_id,
    ADD COLUMN default_organization_id BIGINT UNSIGNED GENERATED ALWAYS AS (IF(is_default = 1, organization_id, NULL)) STORED AFTER default_user_id,
    ADD CONSTRAINT fk_cover_sets_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    ADD UNIQUE KEY uq_personal_cover_set_name (personal_user_id, name),
    ADD UNIQUE KEY uq_organization_cover_set_name (organization_id, name),
    ADD UNIQUE KEY uq_one_personal_default_cover (default_user_id),
    ADD UNIQUE KEY uq_one_organization_default_cover (default_organization_id),
    ADD INDEX idx_cover_organization_default (organization_id, is_default);

ALTER TABLE audit_events
    ADD COLUMN organization_id BIGINT UNSIGNED NULL AFTER user_id,
    ADD CONSTRAINT fk_audit_organization FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL;

CREATE TABLE user_preferences (
    user_id BIGINT UNSIGNED PRIMARY KEY,
    tool_order_json JSON NULL,
    recommended_tools_json JSON NULL,
    menu_tool_order_json JSON NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    CONSTRAINT fk_preferences_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
