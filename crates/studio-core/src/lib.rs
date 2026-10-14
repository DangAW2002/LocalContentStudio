pub mod actor;
pub mod asset;
pub mod brand;
pub mod capabilities;
pub mod document;
pub mod error;
pub mod handoff;
pub mod media;
pub mod operation;
pub mod preview;
pub mod project;
pub mod research;
pub mod session;

pub use actor::{ActorContext, ActorRole};
pub use asset::{Asset, AssetKind, AssetScope, RightsAttestation, RightsState};
pub use brand::{
    BrandKit, BrandVersion, BrandVersionState, ChannelVariant, ContentBrief, VisualTokens,
    VoiceGuidelines,
};
pub use capabilities::{CapabilityItem, CapabilityRegistry, CapabilityStatus};
pub use document::{
    Artboard, ArtboardBackground, BoundingBox, BrandRef, CreativeDocument, Element,
    ElementSemanticRole, ElementStyle, ElementType, TemplateBinding,
};
pub use error::{ErrorCode, ErrorEnvelope, StudioError};
pub use handoff::{
    DeliveryMediaItem, DeliveryPackage, DeliveryTargetSelectors, DeliveryVariant, HandoffStatus,
};
pub use media::{RenderJob, RenderJobState, StoryboardScene, Timeline};
pub use operation::{OperationEnvelope, OperationResult, SideEffectClass};
pub use preview::{ExportReceipt, ExportedFileItem, PreviewFidelity, PreviewReceipt};
pub use project::{Project, ProjectRevision, ProjectStatus, ProjectType};
pub use research::{EvidenceSpan, ResearchSnapshot, ResearchSource};
pub use session::{AuthenticatedSession, HumanConfirmationProof};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_roles() {
        let human = ActorContext::new_human("user-1", "ws-1");
        assert!(human.is_human());
        assert!(!human.is_agent());

        let agent = ActorContext::new_agent("agent-1", "ws-1");
        assert!(agent.is_agent());
        assert!(!agent.is_human());
    }

    #[test]
    fn test_creative_document_hashing() {
        let mut doc = CreativeDocument::new("doc-test");
        let hash1 = doc.compute_hash();
        assert_eq!(hash1.len(), 64);

        doc.artboards.push(Artboard {
            id: "ab-1".into(),
            width: 1080,
            height: 1080,
            background: ArtboardBackground {
                token: "brand.surface".into(),
            },
            elements: vec![],
        });
        let hash2 = doc.compute_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_error_envelope_conversion() {
        let err = StudioError::ConflictCas {
            expected: "rev-1".into(),
            current: "rev-2".into(),
            correlation_id: "corr-123".into(),
        };
        let env = err.to_envelope();
        assert_eq!(env.code, ErrorCode::ConflictCas);
        assert!(env.retryable);
    }

    #[test]
    fn test_capabilities_human_only() {
        let reg = CapabilityRegistry::default();
        assert!(reg.is_human_only("rights.attest"));
        assert!(reg.is_human_only("publish.confirm"));
        assert!(!reg.is_human_only("export.canvas.png"));
    }
}
