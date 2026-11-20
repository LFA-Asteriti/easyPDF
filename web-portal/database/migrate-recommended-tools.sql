-- Run once on installations whose user_preferences table predates customizable recommendations.
ALTER TABLE user_preferences
    ADD COLUMN IF NOT EXISTS recommended_tools_json JSON NULL AFTER tool_order_json;
