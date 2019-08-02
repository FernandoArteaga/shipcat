use merge::Merge;
use std::collections::BTreeMap;

use shipcat_definitions::structs::{
    autoscaling::AutoScaling, security::DataHandling, tolerations::Tolerations, volume::Volume,
    ConfigMap, Dependency, Gate, HealthCheck, HostAlias,
    Kafka, LifeCycle, Metadata, PersistentVolume, Port, Probe, Rbac,
    RollingUpdate, VaultOpts, VolumeMount,
};
use shipcat_definitions::{Config, Manifest, BaseManifest, Region, Result};

use super::{SimpleManifest};
use super::container::{ContainerBuildParams, CronJobSource, JobSource, SidecarSource, InitContainerSource, EnvVarsSource, WorkerSource, ResourceRequirementsSource, ImageNameSource, ImageTagSource};
use super::kong::{KongSource, KongBuildParams};
use super::util::{Build, Enabled, RelaxedString, Require};

/// Main manifest, deserialized from `manifest.yml`
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ManifestSource {
    pub name: Option<String>,
    pub external: bool,
    pub disabled: bool,
    pub regions: Vec<String>,
    pub metadata: Option<Metadata>,

    #[serde(flatten)]
    pub overrides: ManifestOverrides,
}

/// Manifest overrides, deserialized from `dev-uk.yml`/`prod.yml` etc.
#[derive(Deserialize, Default, Merge, Clone)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestOverrides {
    pub publicly_accessible: Option<bool>,
    pub image: Option<ImageNameSource>,
    pub image_size: Option<u32>,
    pub version: Option<ImageTagSource>,
    pub command: Option<Vec<String>>,
    pub data_handling: Option<DataHandling>,
    pub language: Option<String>,
    pub resources: Option<ResourceRequirementsSource>,
    pub secret_files: BTreeMap<String, String>,
    pub configs: Option<ConfigMap>,
    pub vault: Option<VaultOpts>,
    pub http_port: Option<u32>,
    pub ports: Option<Vec<Port>>,
    pub external_port: Option<u32>,
    pub health: Option<HealthCheck>,
    pub dependencies: Option<Vec<Dependency>>,
    pub workers: Option<Vec<WorkerSource>>,
    pub sidecars: Option<Vec<SidecarSource>>,
    pub readiness_probe: Option<Probe>,
    pub liveness_probe: Option<Probe>,
    pub lifecycle: Option<LifeCycle>,
    pub rolling_update: Option<RollingUpdate>,
    pub auto_scaling: Option<AutoScaling>,
    pub tolerations: Option<Vec<Tolerations>>,
    pub host_aliases: Option<Vec<HostAlias>>,
    pub init_containers: Option<Vec<InitContainerSource>>,
    pub volumes: Option<Vec<Volume>>,
    pub volume_mounts: Option<Vec<VolumeMount>>,
    pub persistent_volumes: Option<Vec<PersistentVolume>>,
    pub cron_jobs: Option<Vec<CronJobSource>>,
    pub jobs: Option<Vec<JobSource>>,
    pub service_annotations: BTreeMap<String, String>,
    pub pod_annotations: BTreeMap<String, RelaxedString>,
    pub labels: BTreeMap<String, RelaxedString>,
    pub gate: Option<Gate>,
    pub hosts: Option<Vec<String>>,
    pub kafka: Option<Kafka>,
    pub source_ranges: Option<Vec<String>>,
    pub rbac: Option<Vec<Rbac>>,

    #[serde(flatten)]
    pub defaults: ManifestDefaults,
}

/// Global/regional manifest defaults, deserialized from `shipcat.conf` etc.
#[derive(Deserialize, Default, Merge, Clone)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestDefaults {
    pub image_prefix: Option<String>,
    pub chart: Option<String>,
    pub replica_count: Option<u32>,
    pub env: EnvVarsSource,
    pub kong: Enabled<KongSource>,
}

impl Build<Manifest, (Config, Region)> for ManifestSource {
    /// Build a Manifest from a ManifestSource, validating and mutating properties.
    fn build(self, (conf, region): &(Config, Region)) -> Result<Manifest> {
        let simple = self.build_simple(conf, region)?;
        let name = simple.base.name;
        let data_handling = self.build_data_handling();
        let kafka = self.build_kafka(&name, region);
        let configs = self.build_configs(&name)?;

        let overrides = self.overrides;
        let defaults = overrides.defaults;

        let container_build_params = ContainerBuildParams {
            main_envs: defaults.env.clone(),
        };

        Ok(Manifest {
            name,
            publiclyAccessible: overrides.publicly_accessible.unwrap_or_default(),
            // TODO: Skip most validation if true
            external: simple.external,
            // TODO: Replace with simple.enabled
            disabled: self.disabled,
            // TODO: Must be non-empty
            regions: simple.base.regions,
            // TODO: Make metadata non-optional
            metadata: Some(simple.base.metadata),
            chart: defaults.chart,
            // TODO: Make imageSize non-optional
            imageSize: overrides.image_size.or(Some(512)),
            image: simple.image,
            version: simple.version,
            command: overrides.command.unwrap_or_default(),
            dataHandling: data_handling,
            language: overrides.language,
            resources: overrides.resources.build(&())?,
            replicaCount: defaults.replica_count,
            env: defaults.env.build(&())?,
            secretFiles: overrides.secret_files,
            configs: configs,
            vault: overrides.vault,
            httpPort: overrides.http_port,
            ports: overrides.ports.unwrap_or_default(),
            externalPort: overrides.external_port,
            health: overrides.health,
            dependencies: overrides.dependencies.unwrap_or_default(),
            workers: overrides.workers.unwrap_or_default().build(&container_build_params)?,
            sidecars: overrides.sidecars.unwrap_or_default().build(&container_build_params)?,
            readinessProbe: overrides.readiness_probe,
            livenessProbe: overrides.liveness_probe,
            lifecycle: overrides.lifecycle,
            rollingUpdate: overrides.rolling_update,
            autoScaling: overrides.auto_scaling,
            tolerations: overrides.tolerations.unwrap_or_default(),
            hostAliases: overrides.host_aliases.unwrap_or_default(),
            initContainers: overrides.init_containers.unwrap_or_default().build(&container_build_params)?,
            volumes: overrides.volumes.unwrap_or_default(),
            volumeMounts: overrides.volume_mounts.unwrap_or_default(),
            persistentVolumes: overrides.persistent_volumes.unwrap_or_default(),
            cronJobs: overrides.cron_jobs.unwrap_or_default().build(&container_build_params)?,
            jobs: overrides.jobs.unwrap_or_default().build(&container_build_params)?,
            serviceAnnotations: overrides.service_annotations,
            podAnnotations: overrides.pod_annotations.build(&())?,
            labels: overrides.labels.build(&())?,
            kongApis: simple.kong_apis,
            gate: overrides.gate,
            hosts: overrides.hosts.unwrap_or_default(),
            kafka: kafka,
            sourceRanges: overrides.source_ranges.unwrap_or_default(),
            rbac: overrides.rbac.unwrap_or_default(),

            region: region.name.clone(),
            environment: region.environment.to_string(),
            namespace: region.namespace.clone(),
            secrets: Default::default(),
            kind: Default::default(),
        })
    }
}

impl ManifestSource {
    pub fn build_simple(&self, conf: &Config, region: &Region) -> Result<SimpleManifest> {
        let base = self.build_base(conf)?;

        let overrides = self.overrides.clone();
        let defaults = overrides.defaults;

        Ok(SimpleManifest {
            region: region.name.to_string(),

            enabled: !self.disabled && base.regions.contains(&region.name),
            external: self.external,

            // TODO: Make image non-optional
            image: Some(self.build_image(&base.name)?),

            version: overrides.version.build(&())?,
            kong_apis: defaults.kong.build(&KongBuildParams {
                service: base.name.to_string(),
                region: region.clone(),
                hosts: overrides.hosts,
            })?.unwrap_or_default().map(|k| vec![k]).unwrap_or_default(),

            base,
        })
    }

    pub fn build_base(&self, conf: &Config) -> Result<BaseManifest> {
        // TODO: Remove and use folder name
        let name = self.name.clone().require("name")?;
        let metadata = self.build_metadata(conf)?;
        let regions = self.regions.clone();

        Ok(BaseManifest {
            name,
            regions,
            metadata,
        })
    }

    fn build_image(&self, service: &String) -> Result<String> {
        if let Some(image) = &self.overrides.image {
            image.clone().build(&())
        } else if let Some(prefix) = &self.overrides.defaults.image_prefix {
            Ok(format!("{}/{}", prefix, service))
        } else {
            bail!("Image prefix is not defined")
        }
    }

    // TODO: Extract MetadataSource
    fn build_metadata(&self, conf: &Config) -> Result<Metadata> {
        let mut md = self.metadata.clone().require("metadata")?;

        let team = if let Some(t) = conf.teams.iter().find(|t| t.name == md.team) {
            t
        } else {
            bail!("The team name must match one of the team names in shipcat.conf");
        };
        if md.support.is_none() {
            md.support = team.support.clone();
        }
        if md.notifications.is_none() {
            md.notifications = team.notifications.clone();
        }
        Ok(md.clone())
    }

    // TODO: Extract DataHandlingSource
    fn build_data_handling(&self) -> Option<DataHandling> {
        let original = &self.overrides.data_handling;
        original.clone().map(|mut dh| {
            dh.implicits();
            dh
        })
    }

    // TODO: Extract KafkaSource
    fn build_kafka(&self, service: &String, reg: &Region) -> Option<Kafka> {
        let original = &self.overrides.kafka;
        original.clone().map(|mut kf| {
            kf.implicits(service, reg.clone());
            kf
        })
    }

    // TODO: Extract ConfigsSource
    fn build_configs(&self, service: &String) -> Result<Option<ConfigMap>> {
        let original = &self.overrides.configs;
        if original.is_none() {
            return Ok(None);
        }
        let mut configs = original.clone().unwrap();
        for f in &mut configs.files {
            f.value = Some(read_template_file(service, &f.name)?);
        }
        Ok(Some(configs))
    }

    pub(crate) fn merge_overrides(mut self, other: ManifestOverrides) -> Self {
        self.overrides = self.overrides.merge(other);
        self
    }
}

fn read_template_file(svc: &str, tmpl: &str) -> Result<String> {
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    // try to read file from ./services/{svc}/{tmpl} into `tpl` sting
    let pth = Path::new(".").join("services").join(svc).join(tmpl);
    let gpth = Path::new(".").join("templates").join(tmpl);
    let found_pth = if pth.exists() {
        debug!("Reading template in {}", pth.display());
        pth
    } else {
        if !gpth.exists() {
            bail!(
                "Template {} does not exist in neither {} nor {}",
                tmpl,
                pth.display(),
                gpth.display()
            );
        }
        debug!("Reading template in {}", gpth.display());
        gpth
    };
    // read the template - should work now
    let mut f = File::open(&found_pth)?;
    let mut data = String::new();
    f.read_to_string(&mut data)?;
    Ok(data)
}

impl ManifestDefaults {
    pub(crate) fn merge_source(self, mut other: ManifestSource) -> ManifestSource {
        other.overrides.defaults = self.merge(other.overrides.defaults);
        other
    }
}

#[cfg(test)]
mod tests {
    use merge::Merge;
    use std::collections::BTreeMap;

    use super::ManifestDefaults;

    #[test]
    fn merge() {
        let a = ManifestDefaults {
            image_prefix: Option::Some("alpha".into()),
            chart: Option::None,
            replica_count: Option::Some(1),
            env: {
                let mut env = BTreeMap::new();
                env.insert("a", "default-a");
                env.insert("b", "default-b");
                env.into()
            },
            kong: Default::default(),
        };
        let b = ManifestDefaults {
            image_prefix: Option::Some("beta".into()),
            chart: Option::Some("default".into()),
            replica_count: None,
            env: {
                let mut env = BTreeMap::new();
                env.insert("b", "override-b");
                env.insert("c", "override-c");
                env.into()
            },
            kong: Default::default(),
        };
        let merged = a.merge(b);
        assert_eq!(merged.image_prefix, Option::Some("beta".into()));
        assert_eq!(merged.chart, Option::Some("default".into()));
        assert_eq!(merged.replica_count, Option::Some(1));

        let mut expected_env = BTreeMap::new();
        expected_env.insert("a", "default-a");
        expected_env.insert("b", "override-b");
        expected_env.insert("c", "override-c");
        assert_eq!(merged.env, expected_env.into());
    }
}
