use ckb_types::prelude::IntoTransactionView;

fn main() {
    let exit_0 = std::fs::read("res/exit_0").unwrap();
    let mut dl = ckbez::unittest::Resource::default();
    let mut px = ckbez::unittest::Pickaxer::default();

    let mut tx = ckbez::core::Transaction::default();
    let cell_meta_lock = px.create_cell(&mut dl, 0, ckbez::core::Script::default(), None, &exit_0);
    let cell_meta_i = px.create_cell(&mut dl, 0, px.create_script_by_data(&cell_meta_lock, &[]), None, &[]);
    tx.raw.cell_deps.push(px.create_cell_dep(&cell_meta_lock, 0));
    tx.raw.inputs.push(px.create_cell_input(&cell_meta_i));
    let tx_view = tx.pack().into_view();

    let runner = ckb_script::runner::Runner::new(tx_view, dl, ckb_script::runner::Config::default()).unwrap();
    let result = runner.verify_by_hash(
        ckb_script::ScriptGroupType::Lock,
        &ckb_types::packed::Byte32::new(cell_meta_i.cell_output.lock.hash()),
    );
    println!("verify_by_hash {:?}", result);
    let result = runner.verify_by_location("input".parse().unwrap(), 0, "lock".parse().unwrap());
    println!("verify_by_location {:?}", result);
}
