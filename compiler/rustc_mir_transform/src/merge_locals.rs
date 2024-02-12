use std::collections::hash_map::Entry;
use std::hash::{Hash, Hasher};

use rustc_data_structures::fx::FxHashMap;
use rustc_index::IndexVec;
use rustc_middle::mir::visit::{MutVisitor, PlaceContext};
use rustc_middle::mir::*;
use rustc_middle::ty::TyCtxt;

use crate::ssa::SsaLocals;

pub struct MergeLocals;

impl<'tcx> MirPass<'tcx> for MergeLocals {
    fn is_enabled(&self, sess: &rustc_session::Session) -> bool {
        sess.mir_opt_level() > 1
    }

    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        trace!("Running MergeLocals on {:?}", body.source);
        let mut any_replacement = false;
        while merge_locals(tcx, body) {
            any_replacement = true;
        }
        if any_replacement {
            crate::simplify::remove_unused_definitions(body);
        }
    }
}

fn merge_locals<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) -> bool {
    let ssa = SsaLocals::new(body);
    let mut any_replacement = false;
    let mut same_hashes =
        FxHashMap::with_capacity_and_hasher(body.local_decls().len(), Default::default());
    let mut map = IndexVec::from_elem(None, body.local_decls());
    for data in body.basic_blocks.as_mut_preserves_cfg() {
        for stmt in &data.statements {
            match stmt.kind {
                StatementKind::StorageLive(local) | StatementKind::StorageDead(local) => {
                    map[local] = Some(local);
                }
                StatementKind::Intrinsic(..)
                | StatementKind::Retag(..)
                | StatementKind::Coverage(..)
                | StatementKind::FakeRead(..)
                | StatementKind::PlaceMention(..)
                | StatementKind::AscribeUserType(..)
                | StatementKind::ConstEvalCounter
                | StatementKind::Nop
                | StatementKind::Assign(..)
                | StatementKind::SetDiscriminant { .. }
                | StatementKind::Deinit(..) => {}
            };
        }
    }
    let arg_count = body.arg_count.try_into().unwrap();
    for (local, rvalue, _) in ssa.assignments(body) {
        let ty = body.local_decls[local].ty;
        if local.as_u32() <= arg_count {
            continue;
        }
        // if !ty.is_scalar() {
        //     continue;
        // }
        if local == RETURN_PLACE {
            continue;
        }
        if map[local].is_some() {
            continue;
        }
        let to_hash = RvalueHashable { rvalue };
        let entry = same_hashes.entry(to_hash);
        match entry {
            Entry::Occupied(occupied) => {
                let existed_local = *occupied.get();
                trace!("replacing {:?} with {:?}", local, existed_local);
                map[local] = existed_local;
                any_replacement = true;
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Some(local));
                map[local] = Some(local);
            }
        }
    }
    if !any_replacement {
        return false;
    }

    let mut updater = LocalUpdater { map: &map, tcx };
    updater.visit_body_preserves_cfg(body);
    true
}

struct RvalueHashable<'tcx, 'a> {
    rvalue: &'a Rvalue<'tcx>,
}

impl Hash for RvalueHashable<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rvalue.hash(state);
    }
}

impl Eq for RvalueHashable<'_, '_> {}

impl PartialEq for RvalueHashable<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.rvalue == other.rvalue
    }
}

struct LocalUpdater<'tcx, 'a> {
    map: &'a IndexVec<Local, Option<Local>>,
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> MutVisitor<'tcx> for LocalUpdater<'tcx, '_> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_local(&mut self, l: &mut Local, _: PlaceContext, _: Location) {
        if let Some(new_l) = self.map[*l] {
            if *l != new_l {
                *l = new_l;
            }
        }
    }
}
