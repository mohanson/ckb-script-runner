use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, Resource};
use ckb_script::generate_ckb_syscalls;
use ckb_script::types::{DebugPrinter, Machine, SgData, VmContext, VmId};
use ckb_types::core::cell::CellMeta;
use ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionBuilder};
use ckb_types::packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script};
use ckb_types::prelude::Builder;
use ckb_types::prelude::Entity;
use ckb_vm::registers::{A0, A7};
use ckb_vm::{Bytes, DefaultMachineRunner, Register, SupportMachine, Syscalls};
use std::sync::Arc;

pub struct SyscallCurrentCycles {}

impl SyscallCurrentCycles {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for SyscallCurrentCycles {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let id = machine.registers()[A7].to_u64();
        if id != 2042 {
            return Ok(false);
        }
        machine.set_register(A0, Mac::REG::from_u64(2042));
        return Ok(true);
    }
}

pub fn generate_ckb_syscalls_patch(
    vm_id: &VmId,
    sg_data: &SgData<Resource>,
    vm_context: &VmContext<Resource>,
    _: &u8,
) -> Vec<Box<(dyn Syscalls<<Machine as DefaultMachineRunner>::Inner>)>> {
    let debug_printer: DebugPrinter = Arc::new(|_: &Byte32, message: &str| {
        let message = message.trim_end_matches('\n');
        if message != "" {
            println!("{}", &format!("Script log: {}", message));
        }
    });
    let mut sys_patch = generate_ckb_syscalls(vm_id, sg_data, vm_context, &debug_printer);
    sys_patch.insert(0, Box::new(SyscallCurrentCycles::new()));
    return sys_patch;
}

fn main() {
    let exit_0 = std::fs::read("res/syscall").unwrap();

    let cell_meta_lock_data = Bytes::copy_from_slice(&exit_0);
    let cell_meta_lock = CellMeta {
        cell_output: CellOutput::new_builder()
            .build_exact_capacity(Capacity::bytes(cell_meta_lock_data.len()).unwrap())
            .unwrap(),
        out_point: OutPoint::new(Byte32::from_slice(&vec![0x00; 32]).unwrap(), 0),
        data_bytes: cell_meta_lock_data.len() as u64,
        mem_cell_data: Some(cell_meta_lock_data.clone()),
        mem_cell_data_hash: Some(Byte32::from_slice(&ckb_hash::blake2b_256(&cell_meta_lock_data)).unwrap()),
        ..Default::default()
    };
    let cell_meta_i = CellMeta {
        cell_output: CellOutput::new_builder()
            .lock(
                Script::new_builder()
                    .code_hash(cell_meta_lock.mem_cell_data_hash.unwrap())
                    .hash_type(ScriptHashType::Data2.into())
                    .build(),
            )
            .build_exact_capacity(Capacity::zero())
            .unwrap(),
        out_point: OutPoint::new(Byte32::from_slice(&vec![0x00; 32]).unwrap(), 1),
        ..Default::default()
    };

    let mut mock_info = MockInfo::default();
    mock_info.cell_deps.push(MockCellDep {
        cell_dep: CellDep::new_builder().out_point(cell_meta_lock.out_point).dep_type(DepType::Code.into()).build(),
        output: cell_meta_lock.cell_output,
        data: cell_meta_lock_data.clone(),
        header: None,
    });
    mock_info.inputs.push(MockInput {
        input: CellInput::new(cell_meta_i.out_point, 0),
        output: cell_meta_i.cell_output,
        data: cell_meta_i.mem_cell_data.unwrap_or_default(),
        header: None,
    });

    let tx = TransactionBuilder::default();
    let tx = tx.cell_dep(mock_info.cell_deps[0].cell_dep.clone());
    let tx = tx.input(mock_info.inputs[0].input.clone());
    let tx = tx.build();
    let dl = Resource::from_mock_tx(&MockTransaction { mock_info, tx: tx.data() }).unwrap();

    let config: ckb_script::runner::Config<Resource, u8, ckb_script::types::Machine> = ckb_script::runner::Config {
        max_cycles: 100_000_000,
        syscall_generator: generate_ckb_syscalls_patch,
        syscall_context: 0,
        version: ckb_script::ScriptVersion::V2,
    };
    let runner = ckb_script::runner::Runner::new(tx, dl, config).unwrap();
    let result = runner.verify_by_location("input".parse().unwrap(), 0, "lock".parse().unwrap());
    println!("{:?}", result);
}
