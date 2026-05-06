//! ST runtime macro substitution helpers.
//!
//! 只覆盖当前 RST ST 运行时实际依赖的宏集合，先保证 preset/world info/
//! character/persona 链路与 SillyTavern 的核心行为一致。

use crate::storage::st_resources::TavernCardV3;

use super::runtime_assembly::{STChatMetadata, STChatMessage};

#[derive(Debug, Clone, Default)]
pub struct MacroContext {
    pub user_name: String,
    pub char_name: String,
    pub group_names: String,
    pub persona_name: String,
    pub persona_description: String,
    pub character_description: String,
    pub character_personality: String,
    pub scenario: String,
    pub world_info: String,
    pub original: String,
}

impl MacroContext {
    pub fn from_chat_metadata(
        chat_metadata: &STChatMetadata,
        character: Option<&TavernCardV3>,
        world_info: impl Into<String>,
    ) -> Self {
        let persona = chat_metadata.user_persona.as_ref();
        Self {
            user_name: persona
                .map(|persona| persona.name.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "User".to_string()),
            char_name: character
                .map(|character| character.data.name.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "Character".to_string()),
            group_names: String::new(),
            persona_name: persona
                .map(|persona| persona.name.trim().to_string())
                .unwrap_or_default(),
            persona_description: persona
                .map(|persona| persona.description.trim().to_string())
                .unwrap_or_default(),
            character_description: character
                .map(|character| character.data.description.trim().to_string())
                .unwrap_or_default(),
            character_personality: character
                .map(|character| character.data.personality.trim().to_string())
                .unwrap_or_default(),
            scenario: character
                .map(|character| character.data.scenario.trim().to_string())
                .unwrap_or_default(),
            world_info: world_info.into(),
            original: String::new(),
        }
    }

    pub fn with_world_info(&self, world_info: impl Into<String>) -> Self {
        let mut clone = self.clone();
        clone.world_info = world_info.into();
        clone
    }

    pub fn with_original(&self, original: impl Into<String>) -> Self {
        let mut clone = self.clone();
        clone.original = original.into();
        clone
    }

    pub fn message_speaker_name(&self, message: &STChatMessage) -> String {
        if let Some(name) = message.name.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
            return name.to_string();
        }

        match message.role.as_str() {
            "user" => self.user_name.clone(),
            "assistant" => self.char_name.clone(),
            "system" => "System".to_string(),
            _ => String::new(),
        }
    }
}

pub fn substitute_params(template: &str, context: &MacroContext) -> String {
    if template.is_empty() {
        return String::new();
    }

    template
        .replace("{{user}}", &context.user_name)
        .replace("{{char}}", &context.char_name)
        .replace("{{group}}", &context.group_names)
        .replace("{{persona}}", &context.persona_name)
        .replace("{{persona_description}}", &context.persona_description)
        .replace("{{charDescription}}", &context.character_description)
        .replace("{{description}}", &context.character_description)
        .replace("{{charPersonality}}", &context.character_personality)
        .replace("{{personality}}", &context.character_personality)
        .replace("{{scenario}}", &context.scenario)
        .replace("{{wi}}", &context.world_info)
        .replace("{{world_info}}", &context.world_info)
        .replace("{{original}}", &context.original)
        .replace("{0}", &context.world_info)
}

#[cfg(test)]
mod tests {
    use super::{substitute_params, MacroContext};

    #[test]
    fn substitute_params_replaces_st_runtime_macros() {
        let context = MacroContext {
            user_name: "Alice".to_string(),
            char_name: "Bob".to_string(),
            group_names: "Bob, Eve".to_string(),
            persona_name: "Alice".to_string(),
            persona_description: "A ranger".to_string(),
            character_description: "A knight".to_string(),
            character_personality: "Brave".to_string(),
            scenario: "In a tavern".to_string(),
            world_info: "Lore block".to_string(),
            original: "raw".to_string(),
        };

        let output = substitute_params(
            "{{user}} {{char}} {{group}} {{persona}} {{persona_description}} {{description}} {{personality}} {{scenario}} {{wi}} {0} {{original}}",
            &context,
        );

        assert_eq!(
            output,
            "Alice Bob Bob, Eve Alice A ranger A knight Brave In a tavern Lore block Lore block raw"
        );
    }
}
