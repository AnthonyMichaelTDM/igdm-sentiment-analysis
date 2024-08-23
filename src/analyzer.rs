use std::collections::HashMap;

use vader_sentiment::SentimentIntensityAnalyzer;

use crate::parser::{Message, ParsedConversation, Participant};

pub struct AnalyzedConversation {
    pub analysis: HashMap<Participant, Vec<(Message, Score)>>,
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct Score {
    pub pos: f64,
    pub neu: f64,
    pub neg: f64,
    pub compound: f64,
}

impl ParsedConversation {
    pub fn analyze(&self) -> AnalyzedConversation {
        let analyzer = SentimentIntensityAnalyzer::new();

        let analysis = self
            .participants
            .iter()
            .map(|participant| {
                (
                    participant.clone(),
                    self.messages
                        .iter()
                        .filter(|message| message.sender_name == participant.name)
                        .map(|message| {
                            let scores = analyzer.polarity_scores(&message.content);

                            (
                                message.clone(),
                                Score {
                                    pos: *scores.get("pos").unwrap(),
                                    neu: *scores.get("neu").unwrap(),
                                    neg: *scores.get("neg").unwrap(),
                                    compound: *scores.get("compound").unwrap(),
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        AnalyzedConversation { analysis }
    }
}
