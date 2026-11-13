ALTER TABLE user_preferences
    ADD COLUMN IF NOT EXISTS menu_tool_order_json JSON NULL AFTER recommended_tools_json;
