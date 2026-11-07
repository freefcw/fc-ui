use fc_ui::components::{
    input::ValidationRules,
    textarea::{TextValidationRules, Textarea},
    textarea_state::TextareaState,
};

#[test]
fn textarea_exposes_narrow_rules_without_removing_legacy_entry_points() {
    let rules = TextValidationRules {
        required: true,
        max_length: Some(140),
        ..Default::default()
    };
    let legacy: ValidationRules = rules.clone().into();
    assert!(legacy.required);
    assert_eq!(TextValidationRules::from(legacy).max_length, Some(140));
    let typed_builder: fn(Textarea, TextValidationRules) -> Textarea =
        Textarea::text_validation_rules;
    let legacy_builder: fn(Textarea, ValidationRules) -> Textarea = Textarea::validation_rules;
    let typed_state: fn(TextareaState, TextValidationRules) -> TextareaState =
        TextareaState::text_validation_rules;
    let _ = (typed_builder, legacy_builder, typed_state);
}
