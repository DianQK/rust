//! This pass removes storage markers if they won't be emitted during codegen.

use rustc_hir::def_id::DefId;
use rustc_index::Idx;
use rustc_middle::mir::visit::Visitor;
use rustc_middle::ty::{Instance, InstanceDef, ParamEnv, TyCtxt};
use rustc_middle::{mir::*, ty};

pub struct FunctionAttrs;

impl<'tcx> MirPass<'tcx> for FunctionAttrs {
    fn is_enabled(&self, sess: &rustc_session::Session) -> bool {
        // sess.mir_opt_level() > 0 && !sess.emit_lifetime_markers()
        false
    }

    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        if body.is_polymorphic {
            return;
        }
        info!("Running FunctionAttrs on {:?}", body.source);
        body.fn_attrs.memory_effect = Some(annotate(tcx, body).unwrap_or(MemoryEffect::TOP));
    }
}

fn annotate<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) -> Option<MemoryEffect> {
    let def_id = body.source.def_id().expect_local();
    if !tcx.hir().body_owner_kind(def_id).is_fn_or_closure() {
        return None;
    }
    if body.source.promoted.is_some() {
        return None;
    }
    if body.coroutine.is_some() {
        return None;
    }

    let param_env = tcx.param_env_reveal_all_normalized(def_id);
    let mut annotator =
        AttributeAnnotator { tcx, param_env, history: Vec::new(), value: MemoryEffect::None };
    // if annotator.value == MemoryEffect::TOP {
    //     body.fn_attrs.memory_effect = Some(MemoryEffect::TOP);
    //     return;
    // }
    // let mut updater = MemoryEffectUpdater { value: memory_effect, tcx };
    // updater.visit_body(body);
    // body.fn_attrs.memory_effect = Some(updater.value);
    annotator.process(body, body);
    info!("Memory effect for {:?} is {:?}", body.source, annotator.value);
    Some(annotator.value)
}

struct AttributeAnnotator<'tcx> {
    tcx: TyCtxt<'tcx>,
    param_env: ParamEnv<'tcx>,
    history: Vec<DefId>,
    value: MemoryEffect,
}

impl<'tcx> AttributeAnnotator<'tcx> {
    fn process(&mut self, caller_body: &Body<'tcx>, body: &Body<'tcx>) {
        info!("Processing {:?}", body.source);
        for bb_data in body.basic_blocks.iter() {
            info!("bb_data: {:?}", bb_data);
            if bb_data.is_cleanup {
                continue;
            }

            if let TerminatorKind::Call { ref func, fn_span, .. } = bb_data.terminator().kind {
                info!("func: {:?}", func);
                let Some(callsite) = self.resolve_callsite(caller_body, bb_data) else {
                    warn!("resolve_callsite failed");
                    self.value.join(MemoryEffect::Write);
                    return;
                };

                if let InstanceDef::Intrinsic(..) = callsite.def {
                    continue;
                }
                info!("callsite: {:?}", callsite);

                let callee_mir = self.tcx.instance_mir(callsite.def);
                // info!("callee_mir: {:?}", callee_mir);
                self.process(caller_body, callee_mir);
                continue;
            }
        }
    }

    fn resolve_callsite(
        &self,
        caller_body: &Body<'tcx>,
        bb_data: &BasicBlockData<'tcx>,
    ) -> Option<Instance<'tcx>> {
        // Only consider direct calls to functions
        let terminator = bb_data.terminator();
        if let TerminatorKind::Call { ref func, fn_span, .. } = terminator.kind {
            let func_ty = func.ty(caller_body, self.tcx);
            if let ty::FnDef(def_id, args) = *func_ty.kind() {
                // To resolve an instance its args have to be fully normalized.
                let args = self.tcx.try_normalize_erasing_regions(self.param_env, args).ok()?;
                let callee =
                    Instance::resolve(self.tcx, self.param_env, def_id, args).ok().flatten()?;

                if let InstanceDef::Virtual(..) = callee.def {
                    return None;
                }

                if self.history.contains(&callee.def_id()) {
                    return None;
                }

                return Some(callee);
            }
        }

        None
    }
}

struct MemoryEffectUpdater<'tcx> {
    value: MemoryEffect,
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> Visitor<'tcx> for MemoryEffectUpdater<'tcx> {
    // fn tcx(&self) -> TyCtxt<'tcx> {
    //     self.tcx
    // }

    // fn visit_local(&mut self, l: &mut Local, _: PlaceContext, _: Location) {
    //     *l = self.map[*l].unwrap();
    // }

    fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
        match statement.kind {
            StatementKind::Assign(..) => {
                // TODO: ?
                self.value.join(MemoryEffect::Write);
            }
            StatementKind::FakeRead(..) => {
                self.value.join(MemoryEffect::None);
            }
            StatementKind::SetDiscriminant { .. } => {
                // TODO: ?
                self.value.join(MemoryEffect::Write);
            }
            StatementKind::Deinit(..) => {
                // TODO: ?
                self.value.join(MemoryEffect::Write);
            }
            StatementKind::StorageLive(..)
            | StatementKind::StorageDead(..)
            | StatementKind::Nop
            | StatementKind::ConstEvalCounter => {
                self.value.join(MemoryEffect::None);
            }
            StatementKind::Retag(..) => {
                self.value.join(MemoryEffect::Write);
            }
            StatementKind::PlaceMention(..) => {
                self.value.join(MemoryEffect::Read);
            }
            StatementKind::AscribeUserType(..) => {
                self.value.join(MemoryEffect::None);
            }
            StatementKind::Coverage(..) => {
                self.value.join(MemoryEffect::Write);
            }
            StatementKind::Intrinsic(..) => {
                self.value.join(MemoryEffect::None);
            }
        };
    }

    fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
        match terminator.kind {
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. }
            | TerminatorKind::Assert { .. } => {
                self.value.join(MemoryEffect::None);
            }
            TerminatorKind::Drop { .. } => {
                self.value.join(MemoryEffect::Write);
            }
            TerminatorKind::Call { .. } => {
                self.value.join(MemoryEffect::Write);
            }
            TerminatorKind::Yield { .. } | TerminatorKind::CoroutineDrop => {
                self.value.join(MemoryEffect::Write);
            }
            TerminatorKind::InlineAsm { .. } => {
                self.value.join(MemoryEffect::Write);
            }
        };
    }
}
