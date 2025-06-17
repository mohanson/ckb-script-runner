use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, Resource};
use ckb_script::types::DebugPrinter;
use ckb_types::core::cell::CellMeta;
use ckb_types::core::{Capacity, DepType, ScriptHashType, TransactionBuilder};
use ckb_types::packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script};
use ckb_types::prelude::Builder;
use ckb_types::prelude::Entity;
use ckb_vm::decoder::Decoder;
use ckb_vm::{Bytes, DefaultMachine, DefaultMachineRunner};

#[derive(Clone, Default)]
struct HookCount {
    pub sum: u64,
}

impl<M> ckb_script::runner::Hook<M> for HookCount
where
    M: DefaultMachineRunner,
{
    fn init(_: &DefaultMachine<M::Inner>) -> Self {
        HookCount::default()
    }

    fn init_by_exec(&mut self, _: &M) {}

    fn step(&mut self, _: &mut Decoder, _: &mut M) -> Result<(), ckb_vm::Error> {
        self.sum += 1;
        Ok(())
    }
}

fn main() {
    let exit_0 = std::fs::read("res/exit_0").unwrap();

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
        output: cell_meta_i.cell_output.clone(),
        data: cell_meta_i.mem_cell_data.unwrap_or_default(),
        header: None,
    });

    let tx = TransactionBuilder::default();
    let tx = tx.cell_dep(mock_info.cell_deps[0].cell_dep.clone());
    let tx = tx.input(mock_info.inputs[0].input.clone());
    let tx = tx.build();
    let dl = Resource::from_mock_tx(&MockTransaction { mock_info, tx: tx.data() }).unwrap();

    let config: ckb_script::runner::Config<
        Resource,
        DebugPrinter,
        ckb_script::runner::HookWraper<ckb_script::types::Machine, HookCount>,
    > = ckb_script::runner::Config {
        max_cycles: 100_000_000,
        syscall_generator: ckb_script::generate_ckb_syscalls,
        syscall_context: std::sync::Arc::new(|_: &Byte32, message: &str| {
            let message = message.trim_end_matches('\n');
            if message != "" {
                println!("{}", &format!("Script log: {}", message));
            }
        }),
        version: ckb_script::ScriptVersion::V2,
    };
    let runner = ckb_script::runner::Runner::new(tx, dl, config).unwrap();
    let mut scheduler = runner.get_scheduler_by_location("input".parse().unwrap(), 0, "lock".parse().unwrap()).unwrap();

    let mut record = std::collections::HashMap::new();
    while !scheduler.terminated() {
        let result = scheduler.iterate().unwrap();
        let vm_id = result.executed_vm;
        let hook_count = scheduler.peek(&vm_id, |m| Ok(m.hook.clone()), |&_, &_| unreachable!()).unwrap();
        record.insert(vm_id, hook_count);
    }
    for (k, v) in record {
        println!("{:?} {:?}", k, v.sum);
    }
}
