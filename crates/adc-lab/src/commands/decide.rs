use super::super::*;

pub(crate) fn command_decide_suitability(args: SuitabilityCommand) -> Result<()> {
    let target_contract: TargetOperatingContract = read_json(&args.target_contract)?;
    let workload: WorkloadDemandProfile = read_json(&args.workload_demand)?;
    let policy: SuitabilityPolicy = read_yaml(&args.policy)?;
    let decision = decide_suitability(
        &args.target_run,
        &target_contract,
        &workload,
        &policy,
        path_ref(&args.target_contract),
        path_ref(&args.workload_demand),
        path_ref(&args.policy),
    )?;
    let artifact = suitability_artifact_from_legacy_decision_v2(
        &decision,
        run_id_from_run_dir(&args.target_run),
    );
    write_json_pretty(&args.out, &artifact)?;
    if args.json {
        print_json(&artifact)?;
    } else {
        println!("{}", args.out.display());
    }
    Ok(())
}
