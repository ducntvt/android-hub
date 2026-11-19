//! SDK planning, selection, download, installation and recovery.
mod cache;
mod catalog;
mod download;
mod extract;
pub(crate) mod image_install;
mod install;
pub use image_install::{ImageInstallResult, ImageInstallSpace};
pub(crate) mod package;
mod plan;
mod probe;
mod registration;
mod setup;
mod workflow;

pub use catalog::{ManagedSdkCatalog, ManagedSdkEntry};
pub use download::{SdkArchive, SdkDownloadControl};
pub use extract::{SdkExtraction, SdkExtractionControl};
pub use install::{ManagedSdk, SdkInstallControl, SdkInstallPackage};
pub use package::{SdkPackage, VerifiedSdkPackage, verify_sdk_package, verify_sdk_package_set};
pub use plan::{SdkLicense, SdkPackageSummary, SdkPlan};
pub use probe::SdkToolProbe;
pub use workflow::{SdkSetupControl, SdkSetupStage};
