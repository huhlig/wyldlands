//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! AI components for NPC behavior and personality

use super::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI behavior types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorType {
    Passive,
    Wandering,
    Aggressive,
    Defensive,
    Friendly,
    Merchant,
    Quest,
    Custom,
}

impl BehaviorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BehaviorType::Passive => "Passive",
            BehaviorType::Wandering => "Wandering",
            BehaviorType::Aggressive => "Aggressive",
            BehaviorType::Defensive => "Defensive",
            BehaviorType::Friendly => "Friendly",
            BehaviorType::Merchant => "Merchant",
            BehaviorType::Quest => "Quest",
            BehaviorType::Custom => "Custom",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Passive" => Some(BehaviorType::Passive),
            "Wandering" => Some(BehaviorType::Wandering),
            "Aggressive" => Some(BehaviorType::Aggressive),
            "Defensive" => Some(BehaviorType::Defensive),
            "Friendly" => Some(BehaviorType::Friendly),
            "Merchant" => Some(BehaviorType::Merchant),
            "Quest" => Some(BehaviorType::Quest),
            "Custom" => Some(BehaviorType::Custom),
            _ => None,
        }
    }
}

/// AI state types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateType {
    Idle,
    Moving,
    Combat,
    Fleeing,
    Following,
    Dialogue,
}

impl StateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StateType::Idle => "Idle",
            StateType::Moving => "Moving",
            StateType::Combat => "Combat",
            StateType::Fleeing => "Fleeing",
            StateType::Following => "Following",
            StateType::Dialogue => "Dialogue",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Idle" => Some(StateType::Idle),
            "Moving" => Some(StateType::Moving),
            "Combat" => Some(StateType::Combat),
            "Fleeing" => Some(StateType::Fleeing),
            "Following" => Some(StateType::Following),
            "Dialogue" => Some(StateType::Dialogue),
            _ => None,
        }
    }
}

/// AI controller component
/// Maps to: entity_ai_controller table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIController {
    pub behavior_type: BehaviorType,
    pub current_goal: Option<String>,
    pub state_type: StateType,
    pub state_target_id: Option<EntityId>,
    pub update_interval: f32,
    pub time_since_update: f32,
}

impl AIController {
    /// Create a new AI controller with the given behavior type
    pub fn new(behavior_type: BehaviorType) -> Self {
        Self {
            behavior_type,
            current_goal: None,
            state_type: StateType::Idle,
            state_target_id: None,
            update_interval: 1.0,
            time_since_update: 0.0,
        }
    }
    
    /// Check if the AI should update
    pub fn should_update(&self, _delta_time: f32) -> bool {
        self.time_since_update >= self.update_interval
    }
    
    /// Mark the AI as updated
    pub fn mark_updated(&mut self) {
        self.time_since_update = 0.0;
    }
    
    /// Update the time since last update
    pub fn update_timer(&mut self, delta_time: f32) {
        self.time_since_update += delta_time;
    }
}

/// Emotional State Assessment System (ESAS)
///
/// A dimensional model for representing current emotional states in AI simulations,
/// based on the Scale of Positive and Negative Experience (SPANE) and the Positive
/// and Negative Affect Schedule (PANAS).
///
/// # Core Principles
///
/// **State vs. Trait**: These dimensions measure transient emotional *states* that
/// fluctuate over time, not stable personality *traits*. Values should be updated
/// dynamically in response to events, interactions, and context.
///
/// **Independence**: Dimensions are not mutually exclusive. An entity can simultaneously
/// experience high positive valence and high negative valence (mixed emotions), or high
/// arousal with either positive or negative valence.
///
/// **Measurement Scale**: All dimensions use a 0-1 continuous scale:
/// * 0.0 = Absence/minimum of that emotional quality
/// * 0.5 = Moderate level (where applicable)
/// * 1.0 = Maximum intensity of that emotional quality
///
/// # Theoretical Framework
///
/// ## Core Dimensions:
/// * **Valence**: The hedonic tone of emotion - intrinsic attractiveness (positive)
///   or averseness (negative) of an experience. Based on the fundamental pleasure-
///   displeasure dimension found across all emotion theories.
///
/// * **Arousal**: The activation level of the nervous system - energy, alertness, and
///   physiological mobilization. Independent of whether the experience is pleasant or
///   unpleasant.
///
/// ## Four Quadrants of Core Affect (Russell's Circumplex Model):
/// * **High Arousal + Positive Valence**: Excited, happy, enthusiastic, elated
/// * **High Arousal + Negative Valence**: Angry, fearful, stressed, anxious
/// * **Low Arousal + Positive Valence**: Calm, serene, relaxed, content
/// * **Low Arousal + Negative Valence**: Sad, bored, depressed, lethargic
///
/// ## Specific Affect States:
/// Beyond core valence and arousal, the system tracks specific emotional qualities
/// that have distinct behavioral implications:
/// * **Anxiety**: Apprehensive expectation of threat (future-oriented fear)
/// * **Hostility**: Antagonistic, aggressive orientation toward others
/// * **Engagement**: Cognitive and motivational involvement with environment
/// * **Confidence**: Self-efficacy and perceived control over situations
///
/// # Behavioral Mapping Examples
///
/// ## Approach Behaviors:
/// * `positive_valence: 0.8, arousal: 0.7, engagement: 0.9, confidence: 0.8`
///   → Enthusiastic exploration, proactive goal pursuit
///
/// * `positive_valence: 0.7, arousal: 0.3, engagement: 0.6, confidence: 0.6`
///   → Calm, steady work; contentment without urgency
///
/// ## Avoidance Behaviors:
/// * `negative_valence: 0.7, anxiety: 0.9, confidence: 0.2, arousal: 0.6`
///   → Anxious avoidance, hesitation, flight responses
///
/// * `negative_valence: 0.8, arousal: 0.2, engagement: 0.1, confidence: 0.3`
///   → Withdrawn, depressive avoidance, low motivation
///
/// ## Aggressive Behaviors:
/// * `hostility: 0.9, arousal: 0.9, negative_valence: 0.7, confidence: 0.7`
///   → Active aggression, confrontation, fight responses
///
/// * `hostility: 0.6, arousal: 0.4, negative_valence: 0.5, confidence: 0.3`
///   → Passive-aggressive, irritable but not mobilized
///
/// ## Complex States:
/// * `positive_valence: 0.6, negative_valence: 0.5, arousal: 0.7, anxiety: 0.6`
///   → Mixed emotions: excited but nervous (e.g., before a performance)
///
/// * `engagement: 0.9, arousal: 0.8, positive_valence: 0.8, anxiety: 0.1`
///   → Flow state: absorbed, energized, time distortion
///
/// # Implementation Guidelines
///
/// ## Temporal Dynamics:
/// * **Fast changes**: arousal, anxiety (respond quickly to immediate threats/opportunities)
/// * **Medium changes**: positive_valence, negative_valence, hostility (respond to ongoing situations)
/// * **Slow changes**: confidence, engagement (accumulate over experiences)
///
/// ## Baseline vs. Reactive Values:
/// Consider implementing separate baseline and current values, where current values
/// decay back toward baseline over time in the absence of stimuli.
///
/// ## Interaction Effects:
/// * High `anxiety` typically suppresses `confidence` and `positive_valence`
/// * High `engagement` + high `confidence` amplifies approach behaviors
/// * High `hostility` + low `arousal` may indicate suppressed anger
/// * High `negative_valence` + low `arousal` suggests depression/withdrawal
/// * High `positive_valence` + high `negative_valence` indicates emotional complexity
///
/// ## Validation Considerations:
/// * Values should generally correlate with observable behaviors
/// * Extreme values (>0.9 or <0.1) should be rare and temporary
/// * Most "normal" states cluster around 0.3-0.7 range
/// * Consider clamping or normalizing to prevent unrealistic combinations
pub struct PersonalityMood {
    /// Positive Valence (Happiness/Pleasure)
    ///
    /// Positive valence represents the intrinsic attractiveness and pleasantness of
    /// the current emotional experience. This is the hedonic "good feeling" dimension.
    ///
    /// ## Theoretical Basis:
    /// Derived from SPANE's positive affect items (positive, good, pleasant, happy,
    /// joyful, contented) and PANAS positive affect scale.
    ///
    /// ## Scale Interpretation:
    /// * `0.0` = Neutral hedonic tone; absence of positive feelings
    /// * `0.3-0.5` = Mild pleasant feelings; things are going okay
    /// * `0.5-0.7` = Moderate happiness; satisfied, content
    /// * `0.7-0.9` = Strong positive feelings; very happy, joyful
    /// * `0.9-1.0` = Extreme positive affect; elated, euphoric, ecstatic
    ///
    /// ## Behavioral Implications:
    /// * **High values** (>0.7): Increased approach motivation, social engagement,
    ///   creative thinking, risk tolerance, generosity, optimistic judgments
    /// * **Moderate values** (0.4-0.7): Balanced motivation, openness to opportunities
    /// * **Low values** (<0.3): Reduced motivation (unless driven by other factors),
    ///   neutral or negative expectations
    ///
    /// ## Example States:
    /// * `1.0` - Intense joy at major achievement, peak experience
    /// * `0.8` - Genuinely happy, things are going very well
    /// * `0.5` - Mildly pleasant, comfortable baseline
    /// * `0.2` - Little pleasure, neutral or slightly flat
    /// * `0.0` - Complete absence of positive feelings (not necessarily unhappy)
    ///
    /// ## Implementation Notes:
    /// * Can coexist with moderate `negative_valence` (bittersweet, nostalgia)
    /// * Amplified by success, social connection, novel positive experiences
    /// * Gradually decays toward baseline in absence of positive stimuli
    /// * May temporarily spike during rewards/achievements then normalize
    pub positive_valance: f32,

    /// Negative Valence (Distress/Displeasure)
    ///
    /// Negative valence represents the intrinsic averseness and unpleasantness of
    /// the current emotional experience. This is the hedonic "bad feeling" dimension.
    ///
    /// ## Theoretical Basis:
    /// Derived from SPANE's negative affect items (negative, bad, unpleasant, sad,
    /// afraid, angry) and PANAS negative affect scale.
    ///
    /// ## Scale Interpretation:
    /// * `0.0` = Neutral hedonic tone; absence of negative feelings
    /// * `0.3-0.5` = Mild discomfort; something is bothering the entity
    /// * `0.5-0.7` = Moderate distress; clearly unpleasant experience
    /// * `0.7-0.9` = Strong negative feelings; significant suffering
    /// * `0.9-1.0` = Extreme negative affect; anguish, despair, agony
    ///
    /// ## Behavioral Implications:
    /// * **High values** (>0.7): Strong avoidance motivation, withdrawal, help-seeking,
    ///   defensive behaviors, negative biases in perception and memory
    /// * **Moderate values** (0.4-0.7): Caution, problem-solving focus, reduced social
    ///   interest, attention to threats/problems
    /// * **Low values** (<0.3): No significant distress, comfortable state
    ///
    /// ## Example States:
    /// * `1.0` - Extreme suffering, unbearable emotional pain, crisis
    /// * `0.8` - Significant distress, very upset, major problem
    /// * `0.5` - Moderately bothered, something is wrong, uncomfortable
    /// * `0.2` - Slightly unpleasant, minor annoyance
    /// * `0.0` - No distress, emotionally neutral or positive
    ///
    /// ## Implementation Notes:
    /// * Independent from `positive_valence` - both can be high (complexity, ambivalence)
    /// * Both low indicates emotional numbness or baseline calm
    /// * Elevated by failures, losses, conflicts, threats, pain
    /// * Slower to decay than positive_valence (negativity bias)
    /// * Chronic elevation (>0.6 sustained) may indicate depression/trauma
    pub negative_valance: f32,

    /// Arousal (Activation/Energy)
    ///
    /// Arousal represents the activation level of the organism - the intensity of
    /// physiological and psychological mobilization, independent of hedonic tone.
    ///
    /// ## Theoretical Basis:
    /// Derived from the arousal dimension in dimensional models of emotion (Russell's
    /// circumplex) and PANAS items like "alert," "active," "attentive" vs. their opposites.
    ///
    /// ## Scale Interpretation:
    /// * `0.0-0.2` = Very low arousal; sleepy, sluggish, lethargic
    /// * `0.2-0.4` = Low arousal; calm, relaxed, quiet
    /// * `0.4-0.6` = Moderate arousal; normal waking alertness
    /// * `0.6-0.8` = High arousal; energized, alert, activated
    /// * `0.8-1.0` = Very high arousal; excited, hypervigilant, agitated
    ///
    /// ## Interaction with Valence:
    /// * High arousal + positive valence = Excited, enthusiastic, energetic joy
    /// * High arousal + negative valence = Anxious, angry, panicked, stressed
    /// * Low arousal + positive valence = Calm, serene, peaceful, content
    /// * Low arousal + negative valence = Sad, depressed, bored, withdrawn
    ///
    /// ## Behavioral Implications:
    /// * **High arousal** (>0.7): Rapid reactions, heightened sensory processing,
    ///   increased motor activity, difficulty with sustained attention, energized
    ///   behaviors (whether approach or avoidance depends on valence)
    /// * **Moderate arousal** (0.4-0.6): Optimal for complex cognitive tasks,
    ///   balanced responsiveness, normal engagement
    /// * **Low arousal** (<0.3): Reduced reactivity, slower responses, difficulty
    ///   initiating action, may indicate fatigue or depression
    ///
    /// ## Example States:
    /// * `1.0` - Extreme activation; panic, rage, intense excitement, peak performance
    /// * `0.8` - Very energized; highly alert, ready for intense activity
    /// * `0.5` - Normal waking state; comfortably alert
    /// * `0.3` - Relaxed; low energy but conscious
    /// * `0.1` - Nearly asleep; barely conscious, exhausted
    ///
    /// ## Implementation Notes:
    /// * Changes quickly in response to immediate stimuli
    /// * Follows circadian rhythms (naturally higher during day, lower at night)
    /// * Inverted-U relationship with performance (moderate arousal = best performance)
    /// * Can be temporarily elevated by threats, opportunities, stimulants
    /// * Chronic high arousal (>0.7) may indicate stress or anxiety disorders
    pub arousal: f32,

    /// Anxiety/Nervousness
    ///
    /// Anxiety represents apprehensive expectation about potential future threats,
    /// characterized by worry, tension, and physiological activation oriented toward
    /// detecting and avoiding danger.
    ///
    /// ## Theoretical Basis:
    /// Derived from PANAS items "nervous," "scared," "afraid," "jittery" and anxiety
    /// subscales in clinical emotion measures. Distinct from fear (which is response
    /// to present threat) by its future orientation and uncertainty.
    ///
    /// ## Scale Interpretation:
    /// * `0.0-0.2` = Completely calm; no worry or apprehension
    /// * `0.2-0.4` = Mild concern; slight unease about future
    /// * `0.4-0.6` = Moderate anxiety; noticeable worry, some tension
    /// * `0.6-0.8` = High anxiety; significant worry, difficulty relaxing
    /// * `0.8-1.0` = Extreme anxiety; panic, terror, overwhelming dread
    ///
    /// ## Relationship to Other Dimensions:
    /// * Typically accompanied by elevated `arousal` (physiological activation)
    /// * Usually increases `negative_valence` and decreases `positive_valence`
    /// * Inversely related to `confidence` (anxiety reflects low perceived control)
    /// * May increase or decrease `engagement` depending on anxiety type
    ///
    /// ## Behavioral Implications:
    /// * **High anxiety** (>0.7): Avoidance behaviors, hypervigilance, difficulty
    ///   making decisions, seeking reassurance, withdrawal from novel situations,
    ///   catastrophic thinking, attention bias toward threats
    /// * **Moderate anxiety** (0.4-0.6): Cautious approach, increased checking
    ///   behaviors, can motivate preparation but may impair performance
    /// * **Low anxiety** (<0.3): Relaxed, willing to take reasonable risks,
    ///   open to novel experiences
    ///
    /// ## Example States:
    /// * `1.0` - Panic attack; overwhelming terror, feeling of impending doom
    /// * `0.8` - Severe anxiety; can't stop worrying, physical symptoms intense
    /// * `0.6` - Notably anxious; worried about specific concerns, tense
    /// * `0.4` - Mild nervousness; slightly on edge, manageable concern
    /// * `0.2` - Minimal worry; mostly at ease
    /// * `0.0` - Completely calm; no apprehension whatsoever
    ///
    /// ## Implementation Notes:
    /// * Increases rapidly in response to uncertain/unpredictable situations
    /// * Amplified by lack of control, ambiguous threats, social evaluation
    /// * Decays slowly; tends to maintain elevation even after threat passes
    /// * Chronic elevation (>0.5 sustained) indicates anxiety as a trait-like pattern
    /// * May trigger specific phobic responses (>0.8) to particular stimuli
    pub anxiety: f32,

    /// Hostility/Aggression
    ///
    /// Hostility represents antagonistic orientation toward others, ranging from
    /// irritation and resentment to rage and violent impulses. Combines angry affect
    /// with aggressive motivational tendency.
    ///
    /// ## Theoretical Basis:
    /// Derived from PANAS items "hostile," "irritable," and anger subscales in emotion
    /// measures. Represents the anger-aggression dimension distinct from other negative
    /// emotions like sadness or anxiety.
    ///
    /// ## Scale Interpretation:
    /// * `0.0-0.2` = Peaceful; no irritation or antagonism
    /// * `0.2-0.4` = Slight irritation; minor annoyance, easily managed
    /// * `0.4-0.6` = Moderate hostility; noticeably irritated, assertive
    /// * `0.6-0.8` = High hostility; angry, aggressive impulses, confrontational
    /// * `0.8-1.0` = Extreme hostility; rage, violent impulses, loss of control
    ///
    /// ## Relationship to Other Dimensions:
    /// * Typically accompanied by elevated `arousal` (anger is activating emotion)
    /// * Usually increases `negative_valence` but not always (some enjoy aggression)
    /// * Often paired with moderate-to-high `confidence` (approach-oriented emotion)
    /// * Inversely related to `anxiety` (anger = fight; anxiety = flight)
    ///
    /// ## Behavioral Implications:
    /// * **High hostility** (>0.7): Aggressive approach, confrontation, verbal/physical
    ///   attacks, dominance behaviors, reduced empathy, hostile attributions about
    ///   others' intentions, impulsive reactive aggression
    /// * **Moderate hostility** (0.4-0.6): Assertiveness, standing ground, irritable
    ///   responses, competitive behavior, reduced cooperation
    /// * **Low hostility** (<0.3): Peaceful, cooperative, tolerant, prosocial
    ///
    /// ## Example States:
    /// * `1.0` - Murderous rage; violent impulses, total loss of control
    /// * `0.8` - Furious; intense anger, strong desire to attack/harm
    /// * `0.6` - Very angry; clearly hostile, confrontational, aggressive
    /// * `0.4` - Irritated; annoyed, slightly confrontational
    /// * `0.2` - Mildly annoyed; barely noticeable irritation
    /// * `0.0` - Completely peaceful; no antagonism whatsoever
    ///
    /// ## Implementation Notes:
    /// * Increases in response to frustration, perceived injustice, threats to status
    /// * Can be triggered by blocked goals, disrespect, territorial intrusions
    /// * Amplified by low `confidence` → `anxiety` → displaced aggression
    /// * Decays relatively quickly unless sustained by ongoing provocations
    /// * Chronic elevation (>0.4 sustained) may indicate hostile attribution bias
    /// * Consider separate reactive (hot) vs. instrumental (cold) aggression
    pub hostility: f32,

    /// Interest/Engagement
    ///
    /// Engagement represents the degree of cognitive and motivational involvement with
    /// the environment - curiosity, attentiveness, absorption, and desire to interact
    /// with or explore situations.
    ///
    /// ## Theoretical Basis:
    /// Derived from PANAS items "interested," "alert," "attentive," "inspired" and
    /// related to intrinsic motivation and flow state concepts.
    ///
    /// ## Scale Interpretation:
    /// * `0.0-0.2` = Apathetic; no interest, completely disengaged, bored
    /// * `0.2-0.4` = Mild interest; somewhat attentive, minimal engagement
    /// * `0.4-0.6` = Moderate interest; reasonably engaged, normal attention
    /// * `0.6-0.8` = High interest; captivated, absorbed, curious
    /// * `0.8-1.0` = Extreme interest; completely absorbed, flow state, inspired
    ///
    /// ## Relationship to Other Dimensions:
    /// * Often (but not always) accompanies `positive_valence`
    /// * Typically requires moderate `arousal` (alert enough to attend)
    /// * Enhanced by `confidence` (more likely to engage when capable)
    /// * Suppressed by high `anxiety` (threat focus) or `negative_valence` (withdrawal)
    ///
    /// ## Behavioral Implications:
    /// * **High engagement** (>0.7): Exploratory behavior, sustained attention,
    ///   intrinsic motivation, creative problem-solving, seeking information,
    ///   time distortion (flow), resistance to distraction
    /// * **Moderate engagement** (0.4-0.6): Normal attention and participation,
    ///   responsive to novelty, adequate task focus
    /// * **Low engagement** (<0.3): Boredom, apathy, difficulty sustaining attention,
    ///   seeking external stimulation, avoidance of demands, passivity
    ///
    /// ## Example States:
    /// * `1.0` - Complete absorption; flow state, time disappears, peak engagement
    /// * `0.8` - Fascinated; deeply curious, can't look away, inspired
    /// * `0.6` - Interested; actively engaged, attentive, curious
    /// * `0.4` - Mildly engaged; paying attention but not captivated
    /// * `0.2` - Slightly bored; minimal interest, attention wandering
    /// * `0.0` - Completely apathetic; total disengagement, profound boredom
    ///
    /// ## Implementation Notes:
    /// * Increases in response to novelty, complexity, personal relevance
    /// * Enhanced by optimal challenge level (not too easy, not too hard)
    /// * Requires sufficient `arousal` (can't be engaged while exhausted)
    /// * Amplified by progress toward goals and positive feedback
    /// * Chronic low engagement (<0.3) may indicate depression or burnout
    /// * Peak engagement (>0.8) + moderate challenge = flow state conditions
    pub engagement: f32,

    /// Confidence/Self-Efficacy
    ///
    /// Confidence represents perceived competence and control - the belief that one
    /// can successfully handle situations and achieve desired outcomes. Combines
    /// self-efficacy with feelings of strength and determination.
    ///
    /// ## Theoretical Basis:
    /// Related to PANAS items "strong," "determined," "active" and self-efficacy
    /// theory. Represents the empowerment-powerlessness dimension of emotional
    /// experience.
    ///
    /// ## Scale Interpretation:
    /// * `0.0-0.2` = Powerless; helpless, incompetent, no control
    /// * `0.2-0.4` = Low confidence; insecure, doubting abilities
    /// * `0.4-0.6` = Moderate confidence; capable but uncertain
    /// * `0.6-0.8` = High confidence; self-assured, competent, in control
    /// * `0.8-1.0` = Extreme confidence; invincible feeling, total self-assurance
    ///
    /// ## Relationship to Other Dimensions:
    /// * Often accompanies `positive_valence` (success feels good)
    /// * Inversely related to `anxiety` (confidence = control; anxiety = lack of control)
    /// * Enables high `engagement` (more willing to engage when confident)
    /// * Moderates relationship between `hostility` and action (confident hostility → aggression)
    ///
    /// ## Behavioral Implications:
    /// * **High confidence** (>0.7): Approach behaviors, taking initiative, risk-taking,
    ///   assertiveness, persistence in face of obstacles, acceptance of challenges,
    ///   leadership behaviors, resilience to setbacks
    /// * **Moderate confidence** (0.4-0.6): Balanced approach, reasonable risk
    ///   assessment, can attempt challenges with some hesitation
    /// * **Low confidence** (<0.3): Avoidance of challenges, help-seeking, passivity,
    ///   giving up easily, hesitation, deferring to others
    ///
    /// ## Example States:
    /// * `1.0` - Absolutely certain of success; invincible feeling, unstoppable
    /// * `0.8` - Very confident; strong self-belief, ready for any challenge
    /// * `0.6` - Confident; self-assured, capable, in control
    /// * `0.4` - Somewhat unsure; can try but doubting abilities
    /// * `0.2` - Insecure; feeling inadequate, low self-belief
    /// * `0.0` - Completely powerless; utterly helpless, total self-doubt
    ///
    /// ## Implementation Notes:
    /// * Increases with success experiences, positive feedback, mastery
    /// * Decreases with failures, criticism, loss of control
    /// * Domain-specific (can be confident in one area, not another)
    /// * Changes more slowly than state emotions (semi-stable)
    /// * Moderate confidence often optimal (overconfidence = recklessness)
    /// * Chronic low confidence (<0.3) may indicate learned helplessness
    /// * Extremely high confidence (>0.9) may indicate mania or delusion
    pub confidence: f32,
}

/// Personality component for LLM context
/// Maps to: entity_personality table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub background: String,
    pub speaking_style: String,
}

impl Personality {
    /// Create a new personality
    pub fn new() -> Self {
        Self {
            background: String::new(),
            speaking_style: "neutral".to_string(),
        }
    }
    
    /// Set the background story
    pub fn with_background(mut self, background: String) -> Self {
        self.background = background;
        self
    }
    
    /// Set the speaking style
    pub fn with_speaking_style(mut self, speaking_style: String) -> Self {
        self.speaking_style = speaking_style;
        self
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::new()
    }
}

/// BigFive personality profile
/// Maps to: entity_personality_bigfive table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityBigFive {
    /// Neuroticism (0-120)
    ///
    /// Neuroticism refers to the tendency to experience negative feelings.
    ///
    /// Freud originally used the term neurosis to describe a condition marked by mental distress,
    /// emotional suffering, and an inability to cope effectively with the normal demands of life.
    /// He suggested that everyone shows some signs of neurosis, but that we differ in our degree
    /// of suffering and our specific symptoms of distress. Today neuroticism refers to the tendency
    /// to experience negative feelings.
    ///
    /// Those who score high on Neuroticism may experience primarily one specific negative feeling
    /// such as anxiety, anger, or depression, but are likely to experience several of these
    /// emotions.
    ///
    /// People high in neuroticism are emotionally reactive. They respond emotionally to events
    /// that would not affect most people, and their reactions tend to be more intense than normal.
    /// They are more likely to interpret ordinary situations as threatening, and minor frustrations
    /// as hopelessly difficult.
    ///
    /// Their negative emotional reactions tend to persist for unusually long periods of time,
    /// which means they are often in a bad mood. These problems in emotional regulation can
    /// diminish a neurotic's ability to think clearly, make decisions, and cope effectively
    /// with stress.
    pub neuroticism: i32,
    /// Anxiety (0-20)
    ///
    /// The "fight-or-flight" system of the brain of anxious individuals is too easily and too often
    /// engaged. Therefore, people who are high in anxiety often feel like something dangerous is
    /// about to happen. They may be afraid of specific situations or be just generally fearful.
    /// They feel tense, jittery, and nervous. Persons low in Anxiety are generally calm and
    /// fearless.
    pub anxiety: i32,
    /// Anger (0-20)
    ///
    /// Persons who score high in Anger feel enraged when things do not go their way. They are
    /// sensitive about being treated fairly and feel resentful and bitter when they feel they are
    /// being cheated. This scale measures the tendency to feel angry; whether or not the person
    /// expresses annoyance and hostility depends on the individual's level on Agreeableness. Low
    /// scorers do not get angry often or easily.
    pub anger: i32,
    /// Depression (0-20)
    ///
    /// This scale measures the tendency to feel sad, dejected, and discouraged. High scorers lack
    /// energy and have difficulty initiating activities. Low scorers tend to be free from these
    /// depressive feelings.
    pub depression: i32,
    /// Self-Consciousness (0-20)
    ///
    /// Self-conscious individuals are sensitive about what others think of them. Their concern
    /// about rejection and ridicule cause them to feel shy and uncomfortable around others. They
    /// are easily embarrassed and often feel ashamed. Their fears that others will criticize or
    /// make fun of them are exaggerated and unrealistic, but their awkwardness and discomfort may
    /// make these fears a self-fulfilling prophecy. Low scorers, in contrast, do not suffer from
    /// the mistaken impression that everyone is watching and judging them. They do not feel
    /// nervous in social situations.
    pub self_consciousness: i32,
    /// Immoderation (0-20)
    ///
    /// Immoderate individuals feel strong cravings and urges that they have difficulty resisting.
    /// They tend to be oriented toward short-term pleasures and rewards rather than long-term
    /// consequences. Low scorers do not experience strong, irresistible cravings and consequently
    /// do not find themselves tempted to overindulge.
    pub immoderation: i32,
    /// Vulnerability (0-20)
    ///
    /// High scorers on Vulnerability experience panic, confusion, and helplessness when under
    /// pressure or stress. Low scorers feel more poised, confident, and clear-thinking when
    /// stressed.
    pub vulnerability: i32,
    
    /// Extroversion (0-120)
    ///
    /// Extroversion is marked by pronounced engagement with the external world.
    ///
    /// Extraverts enjoy being with people, are full of energy, and often experience positive
    /// emotions. They tend to be enthusiastic, action-oriented, individuals who are likely to say
    /// "Yes!" or "Let's go!" to opportunities for excitement. In groups they like to talk, assert
    /// themselves, and draw attention to themselves.
    ///
    /// Introverts lack the exuberance, energy, and activity levels of extraverts. They tend to be
    /// quiet, low-key, deliberate, and disengaged from the social world. Their lack of social
    /// involvement should not be interpreted as shyness or depression; the introvert simply needs
    /// less stimulation than an extravert and prefers to be alone.
    ///
    /// The independence and reserve of the introvert is sometimes mistaken as unfriendliness or
    /// arrogance. In reality, an introvert who scores high on the agreeableness dimension will
    /// not seek others out but will be quite pleasant when approached.
    pub extroversion: i32,
    /// Friendliness (0-20)
    ///
    /// Friendly people genuinely like other people and openly demonstrate positive feelings toward
    /// others. They make friends quickly, and it is easy for them to form close, intimate
    /// relationships. Low scorers on Friendliness are not necessarily cold and hostile, but they
    /// do not reach out to others and are perceived as distant and reserved.
    pub friendliness: i32,
    /// Gregariousness (0-20)
    ///
    /// Gregarious people find the company of others pleasantly stimulating and rewarding. They
    /// enjoy the excitement of crowds. Low scorers tend to feel overwhelmed by, and therefore
    /// actively avoid, large crowds. They do not necessarily dislike being with people sometimes,
    /// but their need for privacy and time to themselves is much greater than for individuals who
    /// score high on this scale.
    pub gregariousness: i32,
    /// Assertiveness (0-20)
    ///
    /// High scorers Assertiveness like to speak out, take charge, and direct the activities of
    /// others. They tend to be leaders in groups. Low scorers tend not to talk much and let others
    /// control the activities of groups.
    pub assertiveness: i32,
    /// Activity Level (0-20)
    ///
    /// Active individuals lead fast-paced, busy lives. They move about quickly, energetically, and
    /// vigorously, and they are involved in many activities. People who score low on this scale
    /// follow a slower and more leisurely, relaxed pace.
    pub activity_level: i32,
    /// Excitement Seeking (0-20)
    ///
    /// High scorers on this scale are easily bored without high levels of stimulation. They love
    /// bright lights and hustle and bustle. They are likely to take risks and seek thrills. Low
    /// scorers are overwhelmed by noise and commotion and are adverse to thrill-seeking.
    pub excitement_seeking: i32,
    /// Cheerfulness (0-20)
    ///
    /// This scale measures positive mood and feelings, not negative emotions (which are a part of
    /// the Neuroticism domain). Persons who score high on this scale typically experience a range
    /// of positive feelings, including happiness, enthusiasm, optimism, and joy. Low scorers are
    /// not as prone to such energetic, high spirits.
    pub cheerfulness: i32,
    
    /// Openness (0-120)
    ///
    /// Openness to Experience describes a dimension of cognitive style that distinguishes
    /// imaginative, creative people from down-to-earth, conventional people.
    ///
    /// Open people are intellectually curious, appreciative of art, and sensitive to beauty. They
    /// tend to be, compared to closed people, more aware of their feelings. They tend to think and
    /// act in individualistic and nonconforming ways. Intellectuals typically score high on
    /// Openness to Experience; consequently, this factor has also been called Culture or Intellect.
    ///
    /// Nonetheless, Intellect is probably best regarded as one aspect of openness to experience.
    /// Scores on Openness to Experience are only modestly related to years of education and scores
    /// on standard intelligent tests.
    ///
    /// Another characteristic of the open cognitive style is a facility for thinking in symbols
    /// and abstractions far removed from concrete experience. Depending on the individual's
    /// specific intellectual abilities, this symbolic cognition may take the form of mathematical,
    /// logical, or geometric thinking, artistic and metaphorical use of language, music
    /// composition or performance, or one of the many visual or performing arts.
    ///
    /// People with low scores on openness to experience tend to have narrow, common interests. They
    /// prefer the plain, straightforward, and obvious over the complex, ambiguous, and subtle. They
    /// may regard the arts and sciences with suspicion, regarding these endeavors as abstruse or of
    /// no practical use. Closed people prefer familiarity over novelty; they are conservative and
    /// resistant to change.
    ///
    /// Openness is often presented as healthier or more mature by psychologists, who are often
    /// themselves open to experience. However, open and closed styles of thinking are useful in
    /// different environments. The intellectual style of the open person may serve a professor
    /// well, but research has shown that closed thinking is related to superior job performance
    /// in police work, sales, and a number of service occupations.
    pub openness: i32,
    /// Imagination (0-20)
    ///
    /// To imaginative individuals, the real world is often too plain and ordinary. High scorers on
    /// this scale use fantasy as a way of creating a richer, more interesting world. Low scorers
    /// are on this scale are more oriented to facts than fantasy.
    pub imagination: i32,
    /// Artistic Interest (0-20)
    ///
    /// High scorers on this scale love beauty, both in art and in nature. They become easily
    /// involved and absorbed in artistic and natural events. They are not necessarily artistically
    /// trained nor talented, although many will be. The defining features of this scale are
    /// interest in, and appreciation of natural and artificial beauty. Low scorers lack aesthetic
    /// sensitivity and interest in the arts.
    pub artistic_interest: i32,
    /// Emotionality (0-20)
    ///
    /// Persons high on Emotionality have good access to and awareness of their own feelings. Low
    /// scorers are less aware of their feelings and tend not to express their emotions openly.
    pub emotionality: i32,
    /// Adventurousness (0-20)
    ///
    /// High scorers on adventurousness are eager to try new activities, travel to foreign lands,
    /// and experience different things. They find familiarity and routine boring, and will take a
    /// new route home just because it is different. Low scorers tend to feel uncomfortable with
    /// change and prefer familiar routines.
    pub adventurousness: i32,
    /// Intellect (0-20)
    ///
    /// Intellect and artistic interests are the two most important, central aspects of openness to
    /// experience. High scorers on Intellect love to play with ideas. They are open-minded to new
    /// and unusual ideas and like to debate intellectual issues. They enjoy riddles, puzzles, and
    /// brain-teasers. Low scorers on Intellect prefer dealing with either people or things rather
    /// than ideas. They regard intellectual exercises as a waste of time. Intellect should not be
    /// equated with intelligence. Intellect is an intellectual style, not an intellectual ability,
    /// although high scorers on Intellect score slightly higher than low-Intellect individuals on
    /// standardized intelligence tests.
    pub intellect: i32,
    /// Liberalism (0-20)
    ///
    /// Psychological liberalism refers to a readiness to challenge authority, convention, and
    /// traditional values. In its most extreme form, psychological liberalism can even represent
    /// outright hostility toward rules, sympathy for law-breakers, and love of ambiguity, chaos,
    /// and disorder. Psychological conservatives prefer the security and stability brought by
    /// conformity to tradition. Psychological liberalism and conservatism are not identical to
    /// political affiliation, but certainly incline individuals toward certain political parties.
    pub liberalism: i32,
    
    /// Agreeableness (0-120)
    ///
    /// Agreeableness reflects individual differences in concern with cooperation and social
    /// harmony. Agreeable individuals value getting along with others.
    ///
    /// They are therefore considerate, friendly, generous, helpful, and willing to compromise their
    /// interests with others'. Agreeable people also have an optimistic view of human nature. They
    /// believe people are basically honest, decent, and trustworthy.
    ///
    /// Disagreeable individuals place self-interest above getting along with others. They are
    /// generally unconcerned with others' well-being, and therefore are unlikely to extend
    /// themselves for other people. Sometimes their skepticism about others' motives causes them
    /// to be suspicious, unfriendly, and uncooperative.
    ///
    /// Agreeableness is obviously advantageous for attaining and maintaining popularity. Agreeable
    /// people are better liked than disagreeable people. On the other hand, agreeableness is not
    /// useful in situations that require tough or absolute objective decisions. Disagreeable people
    /// can make excellent scientists, critics, or soldiers.
    pub agreeableness: i32,
    /// Trust (0-20)
    ///
    /// A person with high trust assumes that most people are fair, honest, and have good
    /// intentions. Persons low in trust see others as selfish, devious, and potentially dangerous.
    pub trust: i32,
    /// Morality (0-20)
    ///
    /// High scorers on this scale see no need for pretense or manipulation when dealing with
    /// others and are therefore candid, frank, and sincere. Low scorers believe that a certain
    /// amount of deception in social relationships is necessary. People find it relatively easy
    /// to relate to the straightforward high scorers on this scale. They generally find it more
    /// difficult to relate to the unstraightforward low scorers on this scale. It should be made
    /// clear that low scorers are not unprincipled or immoral; they are simply more guarded and
    /// less willing to openly reveal the whole truth.
    pub morality: i32,
    /// Altruism (0-20)
    ///
    /// Altruistic people find helping other people genuinely rewarding. Consequently, they are
    /// generally willing to assist those who are in need. Altruistic people find that doing things
    /// for others is a form of self-fulfillment rather than self-sacrifice. Low scorers on this
    /// scale do not particularly like helping those in need. Requests for help feel like an
    /// imposition rather than an opportunity for self-fulfillment.
    pub altruism: i32,
    /// Cooperation (0-20)
    ///
    /// Individuals who score high on this scale dislike confrontations. They are perfectly willing
    /// to compromise or to deny their own needs in order to get along with others. Those who score
    /// low on this scale are more likely to intimidate others to get their way.
    pub cooperation: i32,
    /// Modesty (0-20)
    ///
    /// High scorers on this scale do not like to claim that they are better than other people. In
    /// some cases this attitude may derive from low self-confidence or self-esteem. Nonetheless,
    /// some people with high self-esteem find immodesty unseemly. Those who are willing to describe
    /// themselves as superior tend to be seen as disagreeably arrogant by other people.
    pub modesty: i32,
    /// Sympathy (0-20)
    ///
    /// People who score high on this scale are tenderhearted and compassionate. They feel the pain
    /// of others vicariously and are easily moved to pity. Low scorers are not affected strongly by
    /// human suffering. They pride themselves on making objective judgments based on reason. They
    /// are more concerned with truth and impartial justice than with mercy.
    pub sympathy: i32,
    
    /// Conscientiousness (0-120)
    ///
    /// Conscientiousness concerns the way in which we control, regulate, and direct our impulses.
    /// Impulses are not inherently bad; occasionally time constraints require a snap decision, and
    /// acting on our first impulse can be an effective response. Also, in times of play rather than
    /// work, acting spontaneously and impulsively can be fun. Impulsive individuals can be seen by
    /// others as colorful, fun-to-be-with, and zany.
    ///
    /// Nonetheless, acting on impulse can lead to trouble in a number of ways. Some impulses are
    /// antisocial. Uncontrolled antisocial acts not only harm other members of society, but also
    /// can result in retribution toward the perpetrator of such impulsive acts. Another problem
    /// with impulsive acts is that they often produce immediate rewards but undesirable, long-term
    /// consequences. Examples include excessive socializing that leads to being fired from one's
    /// job, hurling an insult that causes the breakup of an important relationship, or using
    /// pleasure-inducing drugs that eventually destroy one's health.
    ///
    /// Impulsive behavior, even when not seriously destructive, diminishes a person's effectiveness
    /// in significant ways. Acting impulsively disallows contemplating alternative courses of
    /// action, some of which would have been wiser than the impulsive choice. Impulsivity also
    /// sidetracks people during projects that require organized sequences of steps or stages.
    /// Accomplishments of an impulsive person are therefore small, scattered, and inconsistent.
    ///
    /// A hallmark of intelligence, what potentially separates human beings from earlier life forms,
    /// is the ability to think about future consequences before acting on an impulse. Intelligent
    /// activity involves contemplation of long-range goals, organizing and planning routes to these
    /// goals, and persisting toward one's goals in the face of short-lived impulses to the
    /// contrary. The idea that intelligence involves impulse control is nicely captured by the term
    /// prudence, an alternative label for the Conscientiousness domain. Prudent means both wise and
    /// cautious.
    ///
    /// Persons who score high on the Conscientiousness scale are, in fact, perceived by others as
    /// intelligent. The benefits of high conscientiousness are obvious. Conscientious individuals
    /// avoid trouble and achieve high levels of success through purposeful planning and
    /// persistence. They are also positively regarded by others as intelligent and reliable. On the
    /// negative side, they can be compulsive perfectionists and workaholics. Furthermore, extremely
    /// conscientious individuals might be regarded as stuffy and boring.
    ///
    /// Unconscientious people may be criticized for their unreliability, lack of ambition, and
    /// failure to stay within the lines, but they will experience many short-lived pleasures and
    /// they will never be called stuffy.
    pub conscientiousness: i32,
    /// Self-Efficacy (0-20)
    ///
    /// Self-Efficacy describes confidence in one's ability to accomplish things. High scorers
    /// believe they have the intelligence (common sense), drive, and self-control necessary for
    /// achieving success. Low scorers do not feel effective and may have a sense that they are
    /// not in control of their lives.
    pub self_efficacy: i32,
    /// Orderliness (0-20)
    ///
    /// Persons with high scores on orderliness are well-organized. They like to live according to
    /// routines and schedules. They keep lists and make plans. Low scorers tend to be disorganized
    /// and scattered.
    pub orderliness: i32,
    /// Dutifulness (0-20)
    ///
    /// This scale reflects the strength of a person's sense of duty and obligation. Those who
    /// score high on this scale have a strong sense of moral obligation. Low scorers find
    /// contracts, rules, and regulations overly confining. They are likely to be seen as
    /// unreliable or even irresponsible.
    pub dutifulness: i32,
    /// Achievement Striving (0-20)
    ///
    /// Individuals who score high on this scale strive hard to achieve excellence. Their drive to
    /// be recognized as successful keeps them on track toward their lofty goals. They often have a
    /// strong sense of direction in life, but extremely high scores may be too single-minded and
    /// obsessed with their work. Low scorers are content to get by with a minimal amount of work,
    /// and might be seen by others as lazy.
    pub achievement_striving: i32,
    /// Self-Discipline (0-20)
    ///
    /// Self-discipline-what many people call will-power-refers to the ability to persist at
    /// difficult or unpleasant tasks until they are completed. People who possess high
    /// self-discipline are able to overcome reluctance to begin tasks and stay on track despite
    /// distractions. Those with low self-discipline procrastinate and show poor follow-through,
    /// often failing to complete tasks-even tasks they want very much to complete.
    pub self_discipline: i32,
    /// Cautiousness (0-20)
    ///
    /// Cautiousness describes the disposition to think through possibilities before acting. High
    /// scorers on the Cautiousness scale take their time when making decisions. Low scorers often
    /// say or do the first thing that comes to mind without deliberating alternatives and the
    /// probable consequences of those alternatives.
    pub cautiousness: i32,
}

impl PersonalityBigFive {
    pub fn new() -> Self {
        Self {
            neuroticism: 60,
            anxiety: 10,
            anger: 10,
            depression: 10,
            self_consciousness: 10,
            immoderation: 10,
            vulnerability: 10,
            
            extroversion: 60,
            friendliness: 10,
            gregariousness: 10,
            assertiveness: 10,
            activity_level: 10,
            excitement_seeking: 10,
            cheerfulness: 10,
            
            openness: 60,
            imagination: 10,
            artistic_interest: 10,
            emotionality: 10,
            adventurousness: 10,
            intellect: 10,
            liberalism: 10,
            
            agreeableness: 60,
            trust: 10,
            morality: 10,
            altruism: 10,
            cooperation: 10,
            modesty: 10,
            sympathy: 10,
            
            conscientiousness: 60,
            self_efficacy: 10,
            orderliness: 10,
            dutifulness: 10,
            achievement_striving: 10,
            self_discipline: 10,
            cautiousness: 10,
        }
    }
}

impl Default for PersonalityBigFive {
    fn default() -> Self {
        Self::new()
    }
}

/// Personality traits (key-value pairs)
/// Maps to: entity_personality_traits table (one row per trait)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub traits: HashMap<String, f32>,
}

impl PersonalityTraits {
    pub fn new() -> Self {
        Self {
            traits: HashMap::new(),
        }
    }
    
    /// Set a personality trait (value clamped to -1.0 to 1.0)
    pub fn set_trait(&mut self, trait_name: String, value: f32) {
        self.traits.insert(trait_name, value.clamp(-1.0, 1.0));
    }
    
    /// Get a personality trait (returns 0.0 if not set)
    pub fn get_trait(&self, trait_name: &str) -> f32 {
        self.traits.get(trait_name).copied().unwrap_or(0.0)
    }
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self::new()
    }
}

/// Personality goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityGoal {
    pub goal: String,
    pub priority: i32,
}

/// Personality goals collection
/// Maps to: entity_personality_goals table (one row per goal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityGoals {
    pub goals: Vec<PersonalityGoal>,
}

impl PersonalityGoals {
    pub fn new() -> Self {
        Self {
            goals: Vec::new(),
        }
    }
    
    pub fn add_goal(&mut self, goal: String, priority: i32) {
        self.goals.push(PersonalityGoal { goal, priority });
    }
}

impl Default for PersonalityGoals {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub memory_id: i32,
    pub timestamp: i64,
    pub event: String,
    pub importance: f32,
    pub is_long_term: bool,
    pub entities_involved: Vec<EntityId>,
}

/// Memory system for NPCs
/// Maps to: entity_memory and entity_memory_entities tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub memories: Vec<MemoryEntry>,
    pub next_memory_id: i32,
}

impl Memory {
    /// Create a new memory system
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
            next_memory_id: 1,
        }
    }
    
    /// Add a memory
    pub fn add_memory(&mut self, event: String, importance: f32, entities: Vec<EntityId>) {
        let entry = MemoryEntry {
            memory_id: self.next_memory_id,
            timestamp: 0, // TODO: Use actual timestamp
            event,
            importance,
            is_long_term: importance > 0.7,
            entities_involved: entities,
        };

        self.memories.push(entry);
        self.next_memory_id += 1;
    }
    
    /// Get recent memories
    pub fn get_recent(&self, count: usize) -> &[MemoryEntry] {
        let start = self.memories.len().saturating_sub(count);
        &self.memories[start..]
    }
    
    /// Get important memories
    pub fn get_important(&self, threshold: f32) -> Vec<&MemoryEntry> {
        self.memories.iter()
            .filter(|m| m.importance >= threshold)
            .collect()
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

// Legacy compatibility type (deprecated)
#[deprecated(note = "Use StateType instead")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIState {
    Idle,
    Moving { target: EntityId },
    Combat { target: EntityId },
    Fleeing { from: EntityId },
    Following { target: EntityId },
    Dialogue { with: EntityId },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ai_controller_update() {
        let mut ai = AIController::new(BehaviorType::Wandering);
        assert!(!ai.should_update(0.5));

        ai.update_timer(1.0);
        assert!(ai.should_update(0.0));

        ai.mark_updated();
        assert!(!ai.should_update(0.0));
    }
    
    #[test]
    fn test_personality_traits() {
        let mut traits = PersonalityTraits::new();
        traits.set_trait("friendly".into(), 0.8);
        assert_eq!(traits.get_trait("friendly"), 0.8);
        assert_eq!(traits.get_trait("aggressive"), 0.0);
        
        // Test clamping
        traits.set_trait("extreme".into(), 2.0);
        assert_eq!(traits.get_trait("extreme"), 1.0);
    }
    
    #[test]
    fn test_memory_system() {
        let mut memory = Memory::new();
        
        memory.add_memory("Event 1".into(), 0.5, vec![]);
        memory.add_memory("Event 2".into(), 0.8, vec![]);
        memory.add_memory("Event 3".into(), 0.3, vec![]);
        
        assert_eq!(memory.memories.len(), 3);
        
        let important = memory.get_important(0.75);
        assert_eq!(important.len(), 1);
    }
    
    #[test]
    fn test_personality_bigfive() {
        let bigfive = PersonalityBigFive::new();
        assert_eq!(bigfive.neuroticism, 60);
        assert_eq!(bigfive.extroversion, 60);
    }
}


