use rustc_middle::bug;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_target::abi::call::CastTarget;
use rustc_target::abi::Align;

use super::consts::ConstMethods;
use super::type_::BaseTypeMethods;
use super::{BackendTypes, BuilderMethods, LayoutTypeMethods};

pub trait AbiBuilderMethods<'tcx>: BackendTypes {
    fn get_param(&mut self, index: usize) -> Self::Value;
}

/// The ABI mandates that the value is passed as a different struct representation.
pub trait CastTargetAbiExt<'a, 'tcx, Bx: BuilderMethods<'a, 'tcx>> {
    /// Spill and reload it from the stack to convert from the Rust representation to the ABI representation.
    fn cast_rust_abi_to_other(&self, bx: &mut Bx, src: Bx::Value, align: Align) -> Bx::Value;
    /// Spill and reload it from the stack to convert from the ABI representation to the Rust representation.
    fn cast_other_abi_to_rust(
        &self,
        bx: &mut Bx,
        src: Bx::Value,
        dst: Bx::Value,
        layout: TyAndLayout<'tcx>,
    );
}

impl<'a, 'tcx, Bx: BuilderMethods<'a, 'tcx>> CastTargetAbiExt<'a, 'tcx, Bx> for CastTarget {
    fn cast_rust_abi_to_other(&self, bx: &mut Bx, src: Bx::Value, align: Align) -> Bx::Value {
        let cast_ty = bx.cast_backend_type(self);
        match bx.type_kind(cast_ty) {
            crate::common::TypeKind::Struct | crate::common::TypeKind::Array => {
                let mut index = 0;
                let mut offset = 0;
                let mut target = bx.const_poison(cast_ty);
                for reg in self.prefix.iter().filter_map(|&x| x) {
                    let ptr = if offset == 0 {
                        src
                    } else {
                        bx.inbounds_ptradd(src, bx.const_usize(offset))
                    };
                    let load = bx.load(bx.reg_backend_type(&reg), ptr, align);
                    target = bx.insert_value(target, load, index);
                    index += 1;
                    offset += reg.size.bytes();
                }
                let (rest_count, rem_bytes) = if self.rest.unit.size.bytes() == 0 {
                    (0, 0)
                } else {
                    (
                        self.rest.total.bytes() / self.rest.unit.size.bytes(),
                        self.rest.total.bytes() % self.rest.unit.size.bytes(),
                    )
                };
                for _ in 0..rest_count {
                    let ptr = if offset == 0 {
                        src
                    } else {
                        bx.inbounds_ptradd(src, bx.const_usize(offset))
                    };
                    let load = bx.load(bx.reg_backend_type(&self.rest.unit), ptr, align);
                    target = bx.insert_value(target, load, index);
                    index += 1;
                    offset += self.rest.unit.size.bytes();
                }
                if rem_bytes != 0 {
                    let ptr = bx.inbounds_ptradd(src, bx.const_usize(offset));
                    let load = bx.load(bx.reg_backend_type(&self.rest.unit), ptr, align);
                    target = bx.insert_value(target, load, index);
                }
                target
            }
            ty_kind if bx.type_kind(bx.reg_backend_type(&self.rest.unit)) == ty_kind => {
                bx.load(cast_ty, src, align)
            }
            ty_kind => bug!("cannot cast {ty_kind:?} to the ABI representation in CastTarget"),
        }
    }

    fn cast_other_abi_to_rust(
        &self,
        bx: &mut Bx,
        src: Bx::Value,
        dst: Bx::Value,
        layout: TyAndLayout<'tcx>,
    ) {
        let align = layout.align.abi;
        match bx.type_kind(bx.val_ty(src)) {
            crate::common::TypeKind::Struct | crate::common::TypeKind::Array => {
                let mut index = 0;
                let mut offset = 0;
                for reg in self.prefix.iter().filter_map(|&x| x) {
                    let from = bx.extract_value(src, index);
                    let ptr = if index == 0 {
                        dst
                    } else {
                        bx.inbounds_ptradd(dst, bx.const_usize(offset))
                    };
                    bx.store(from, ptr, align);
                    index += 1;
                    offset += reg.size.bytes();
                }
                let (rest_count, rem_bytes) = if self.rest.unit.size.bytes() == 0 {
                    (0, 0)
                } else {
                    (
                        self.rest.total.bytes() / self.rest.unit.size.bytes(),
                        self.rest.total.bytes() % self.rest.unit.size.bytes(),
                    )
                };
                for _ in 0..rest_count {
                    let from = bx.extract_value(src, index);
                    let ptr = if offset == 0 {
                        dst
                    } else {
                        bx.inbounds_ptradd(dst, bx.const_usize(offset))
                    };
                    bx.store(from, ptr, align);
                    index += 1;
                    offset += self.rest.unit.size.bytes();
                }
                if rem_bytes != 0 {
                    let from = bx.extract_value(src, index);
                    let ptr = bx.inbounds_ptradd(dst, bx.const_usize(offset));
                    bx.store(from, ptr, align);
                }
            }
            ty_kind if bx.type_kind(bx.reg_backend_type(&self.rest.unit)) == ty_kind => {
                let scratch_size = self.unaligned_size(bx);
                let src = if scratch_size > layout.size {
                    // When using just a single register, we directly use load or store instructions,
                    // so we must allocate sufficient space.
                    let scratch_align = self.align(bx);
                    let llscratch = bx.alloca(scratch_size, scratch_align);
                    bx.lifetime_start(llscratch, scratch_size);
                    bx.store(src, llscratch, scratch_align);
                    let tmp = bx.load(bx.backend_type(layout), llscratch, scratch_align);
                    bx.lifetime_end(llscratch, scratch_size);
                    tmp
                } else {
                    src
                };
                bx.store(src, dst, align);
            }
            ty_kind => bug!("cannot cast {ty_kind:?} to the Rust representation in CastTarget"),
        };
    }
}
