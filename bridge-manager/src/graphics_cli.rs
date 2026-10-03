//! Explicit diagnostics use ordinary inspected identities and process custody.
use crate::*;
use graphics::{assessment, pe};

pub(super) fn run(m: &Manager, request_path: &Path) -> Result<()> {
    with_launch_verification(|| {
        // Use the installed inspection host; retained publications and their
        // older audio hosts are not replaced or reinterpreted by a diagnostic.
        let binding = inspection_binding_for(m, read_json(request_path)?, false)?;
        let sw = software(m)?;
        let module_stamp = observation::ModuleStamp::read(&binding.module.path)?;
        let imports = pe::inspect(&mut file(&binding.module.path)?)?;
        binding.module.verify()?;
        let context = assessment::Context {
            module_sha256: binding.module.sha256.clone(),
            class_id: binding.metadata.class_id.clone(),
            runner_fingerprint: catalogue::runner_key(&binding.environment.runner)?,
            environment_fingerprint: hex(&sha2::Sha256::digest(serde_json::to_vec(
                &binding.environment,
            )?)),
            environment_revision: binding.environment.revision,
            host_sha256: binding.host.sha256.clone(),
            host_source_sha256: binding.host_source_sha256.clone(),
            requested_graphics: graphics::requested_backend(
                binding.environment.runner.policy.as_ref(),
            )
            .into(),
        };
        context.fingerprint()?;
        let registry = m.lock("registry.lock")?;
        m.require_inactive(None)?;
        let (mut job, path) = spec(m, binding, true, false, false)?;
        job.graphics_assessment = true;
        atomic_json(&path, &job)?;
        let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
        let child = spawn(m, &sw, &path, None)?;
        drop(registry);
        vendor_product_cli::finish_scan(child, &job, &path, pending)?;
        job.registration.module.verify()?;
        require(
            observation::ModuleStamp::read(&job.registration.module.path)? == module_stamp,
            "graphics_module_changed_during_assessment",
        )?;
        require(
            environment_record(m, &job.registration.environment.id)?
                == job.registration.environment,
            "graphics_environment_changed_during_assessment",
        )?;
        let raw: serde_json::Value = read_json(&job.report)?;
        let result = assessment::assemble(context, imports, &raw, observation::now()?)?;
        let directory = m.root.join("observations/graphics");
        private_dir(&directory)?;
        atomic_json(&directory.join(format!("{}.json", job.session)), &result)?;
        println!("{}", serde_json::to_string(&result)?);
        Ok(())
    })
}
