//! Browser wrapper around Sparrow (https://github.com/JeroenGar/sparrow, MIT).
//!
//! `solve` takes a jagua-rs strip packing instance as JSON (the same format the Sparrow
//! command line reads) and returns the solution as JSON. It blocks until the time budget
//! is spent, so call it from a Web Worker.
use jagua_rs::io::import::Importer;
use jagua_rs::probs::spp::entities::SPSolution;
use jagua_rs::probs::spp::io::ext_repr::ExtSPInstance;
use rand::SeedableRng;
use rand::rngs::Xoshiro256PlusPlus;
use sparrow::EPOCH;
use sparrow::config::*;
use sparrow::consts::{
    DEFAULT_COMPRESS_TIME_RATIO, DEFAULT_EXPLORE_TIME_RATIO, DEFAULT_FAIL_DECAY_RATIO_CMPR,
    DEFAULT_MAX_CONSEQ_FAILS_EXPL,
};
use sparrow::optimizer::optimize;
use sparrow::util::listener::{OptimizationPhase, ReportType, SolutionListener};
use sparrow::util::terminator::BasicTerminator;
use std::time::Duration;
use wasm_bindgen::prelude::*;

struct JsListener {
    cb: Option<js_sys::Function>,
}

impl SolutionListener for JsListener {
    fn report(&mut self, report: ReportType, solution: &SPSolution) {
        let Some(cb) = &self.cb else { return };
        // Only feasible layouts are worth drawing.
        let feasible = matches!(report, ReportType::ExplFeas | ReportType::CmprFeas | ReportType::Final);
        if !feasible {
            return;
        }
        let json = serde_json::to_string(&jagua_rs::probs::spp::io::export(solution, *EPOCH)).unwrap_or_default();
        let _ = cb.call3(
            &JsValue::NULL,
            &JsValue::from_str(&format!("{:?}", report)),
            &JsValue::from_f64(solution.strip_width() as f64),
            &JsValue::from_str(&json),
        );
    }

    fn report_phase(&mut self, phase: OptimizationPhase) {
        if let Some(cb) = &self.cb {
            let _ = cb.call3(
                &JsValue::NULL,
                &JsValue::from_str(&format!("Phase:{:?}", phase)),
                &JsValue::from_f64(0.0),
                &JsValue::from_str(""),
            );
        }
    }
}

/// Runs Sparrow. `time_secs` is the total budget (split between exploration and compression).
/// `progress(kind, stripWidth, solutionJson)` is called when a better feasible layout is found.
#[wasm_bindgen]
pub fn solve(
    instance_json: &str,
    time_secs: f64,
    seed: u32,
    workers: u32,
    progress: Option<js_sys::Function>,
) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    let err = |e: String| JsValue::from_str(&e);

    let ext: ExtSPInstance = serde_json::from_str(instance_json).map_err(|e| err(format!("bad instance: {e}")))?;

    let mut config = DEFAULT_SPARROW_CONFIG;
    let total = Duration::from_secs_f64(time_secs.max(1.0));
    config.expl_cfg.time_limit = total.mul_f32(DEFAULT_EXPLORE_TIME_RATIO);
    config.cmpr_cfg.time_limit = total.mul_f32(DEFAULT_COMPRESS_TIME_RATIO);
    config.expl_cfg.max_conseq_failed_attempts = Some(DEFAULT_MAX_CONSEQ_FAILS_EXPL);
    config.cmpr_cfg.shrink_decay = ShrinkDecayStrategy::FailureBased(DEFAULT_FAIL_DECAY_RATIO_CMPR);
    let n = workers.max(1) as usize;
    config.expl_cfg.separator_config.n_workers = n;
    config.cmpr_cfg.separator_config.n_workers = n;

    let rng = Xoshiro256PlusPlus::seed_from_u64(seed as u64);
    let importer = Importer::new(config.cde_config, config.poly_simpl_tolerance, config.narrow_concavity_cutoff_ratio);
    let instance = jagua_rs::probs::spp::io::import_instance(&importer, &ext).map_err(|e| err(format!("import failed: {e:?}")))?;

    let mut listener = JsListener { cb: progress };
    let mut terminator = BasicTerminator::new();
    let solution = optimize(
        instance,
        rng,
        &mut listener,
        &mut terminator,
        &config.expl_cfg,
        &config.cmpr_cfg,
        None,
    )
    .map_err(|e| err(format!("optimize failed: {e:?}")))?;

    serde_json::to_string(&jagua_rs::probs::spp::io::export(&solution, *EPOCH)).map_err(|e| err(format!("export failed: {e}")))
}
