use {
    crate::common::{rpc_client_utils::send_and_confirm_messages_with_spinner, TestError},
    chrono_humanize::{Accuracy, HumanTime, Tense},
    log::*,
    solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig},
    solana_sdk::{
        account::Account, bpf_loader, bpf_loader_upgradeable::UpgradeableLoaderState,
        commitment_config::CommitmentConfig, loader_instruction, message::Message,
        packet::PACKET_DATA_SIZE, pubkey::Pubkey, rent::Rent, signature::Signature,
        signer::keypair::Keypair, signer::Signer, system_instruction, transaction::Transaction,
    },
    std::{
        fs::File,
        io::Read,
        path::{Path, PathBuf},
    },
};

fn read_file<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let path = path.as_ref();
    let mut file = File::open(path)
        .unwrap_or_else(|err| panic!("Failed to open \"{}\": {}", path.display(), err));

    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)
        .unwrap_or_else(|err| panic!("Failed to read \"{}\": {}", path.display(), err));
    file_data
}

fn find_file(filename: &str) -> Option<PathBuf> {
    for dir in default_shared_object_dirs() {
        let candidate = dir.join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn default_shared_object_dirs() -> Vec<PathBuf> {
    let mut search_path = vec![];
    if let Ok(bpf_out_dir) = std::env::var("BPF_OUT_DIR") {
        search_path.push(PathBuf::from(bpf_out_dir));
    }
    search_path.push(PathBuf::from("tests/fixtures"));
    if let Ok(dir) = std::env::current_dir() {
        search_path.push(dir);
    }
    trace!("BPF .so search path: {:?}", search_path);
    search_path
}

fn calculate_max_chunk_size<F>(create_msg: &F) -> usize
where
    F: Fn(u32, Vec<u8>) -> Message,
{
    let baseline_msg = create_msg(0, Vec::new());
    let tx_size = bincode::serialized_size(&Transaction {
        signatures: vec![
            Signature::default();
            baseline_msg.header.num_required_signatures as usize
        ],
        message: baseline_msg,
    })
    .unwrap() as usize;
    // add 1 byte buffer to account for shortvec encoding
    PACKET_DATA_SIZE.saturating_sub(tx_size).saturating_sub(1)
}

pub fn deploy_program(
    program_name: &str,
    signers: Vec<&dyn Signer>,
    buffer_keypair: &Keypair,
    rpc_client: &RpcClient,
) -> Result<(), TestError> {
    let buffer_signer = buffer_keypair as &dyn Signer;
    let buffer_pubkey = buffer_keypair.pubkey();
    let buffer_authority = buffer_signer;
    let account = add_program(program_name);
    let minimum_balance = rpc_client
        .get_minimum_balance_for_rent_exemption(
            UpgradeableLoaderState::programdata_len(account.data.len())
                .map_err(|e| TestError::UnexpectedError(Box::new(e)))?,
        )
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
    do_process_program_write_and_deploy(
        rpc_client,
        signers,
        &account.data,
        account.data.len(),
        minimum_balance,
        &bpf_loader::id(),
        Some(&[buffer_signer, buffer_authority]),
        Some(buffer_signer),
        &buffer_pubkey,
        buffer_authority,
    )?;
    Ok(())
}

fn do_process_program_write_and_deploy(
    rpc_client: &RpcClient,
    signers: Vec<&dyn Signer>,
    program_data: &[u8],
    buffer_data_len: usize,
    minimum_balance: u64,
    loader_id: &Pubkey,
    program_signers: Option<&[&dyn Signer]>,
    buffer_signer: Option<&dyn Signer>,
    buffer_pubkey: &Pubkey,
    buffer_authority_signer: &dyn Signer,
) -> Result<(), TestError> {
    // Build messages to calculate fees
    let mut messages: Vec<&Message> = Vec::new();

    let initial_instructions = vec![system_instruction::create_account(
        &signers[0].pubkey(),
        buffer_pubkey,
        minimum_balance,
        buffer_data_len as u64,
        loader_id,
    )];
    let initial_message = if !initial_instructions.is_empty() {
        Some(Message::new(
            &initial_instructions,
            Some(&signers[0].pubkey()),
        ))
    } else {
        None
    };

    // Create and add write messages

    let payer_pubkey = signers[0].pubkey();
    let create_msg = |offset: u32, bytes: Vec<u8>| {
        let instruction = loader_instruction::write(buffer_pubkey, loader_id, offset, bytes);
        Message::new(&[instruction], Some(&payer_pubkey))
    };

    let mut write_messages = vec![];
    let chunk_size = calculate_max_chunk_size(&create_msg);
    for (chunk, i) in program_data.chunks(chunk_size).zip(0..) {
        write_messages.push(create_msg((i * chunk_size) as u32, chunk.to_vec()));
    }

    if let Some(ref initial_message) = initial_message {
        messages.push(initial_message);
    }
    let mut write_message_refs = vec![];
    for message in write_messages.iter() {
        write_message_refs.push(message);
    }
    messages.append(&mut write_message_refs);

    // Create and add final message

    let message = Message::new(
        &[loader_instruction::finalize(buffer_pubkey, loader_id)],
        Some(&signers[0].pubkey()),
    );
    let final_message = Some(message);
    if let Some(ref message) = final_message {
        messages.push(message);
    }

    send_deploy_messages(
        rpc_client,
        signers,
        &initial_message,
        &Some(write_messages),
        &final_message,
        buffer_signer,
        Some(buffer_authority_signer),
        program_signers,
    )
    .map_err(|e| TestError::UnexpectedError(e))?;

    Ok(())
}
fn send_deploy_messages(
    rpc_client: &RpcClient,
    signers: Vec<&dyn Signer>,
    initial_message: &Option<Message>,
    write_messages: &Option<Vec<Message>>,
    final_message: &Option<Message>,
    initial_signer: Option<&dyn Signer>,
    write_signer: Option<&dyn Signer>,
    final_signers: Option<&[&dyn Signer]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let payer_signer = signers[0];

    println!("Sending initial message");
    if let Some(message) = initial_message {
        if let Some(initial_signer) = initial_signer {
            trace!("Preparing the required accounts");
            let blockhash = rpc_client.get_latest_blockhash()?;

            let mut initial_transaction = Transaction::new_unsigned(message.clone());
            // Most of the initial_transaction combinations require both the fee-payer and new program
            // account to sign the transaction. One (transfer) only requires the fee-payer signature.
            // This check is to ensure signing does not fail on a KeypairPubkeyMismatch error from an
            // extraneous signature.
            if message.header.num_required_signatures == 2 {
                initial_transaction.try_sign(&[payer_signer, initial_signer], blockhash)?;
            } else {
                initial_transaction.try_sign(&[payer_signer], blockhash)?;
            }
            rpc_client
                .send_and_confirm_transaction_with_spinner_and_config(
                    &initial_transaction,
                    CommitmentConfig::confirmed(),
                    RpcSendTransactionConfig {
                        skip_preflight: true,
                        ..RpcSendTransactionConfig::default()
                    },
                )
                .map_err(|err| format!("Account allocation failed: {}", err))?;
        } else {
            return Err("Buffer account not created yet, must provide a key pair".into());
        }
    }

    println!("Sending write messages");
    if let Some(write_messages) = write_messages {
        if let Some(write_signer) = write_signer {
            trace!("Writing program data");
            let transaction_errors = send_and_confirm_messages_with_spinner(
                rpc_client,
                write_messages,
                &[payer_signer, write_signer],
            )
            .map_err(|err| format!("Data writes to account failed: {}", err))?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

            if !transaction_errors.is_empty() {
                for transaction_error in &transaction_errors {
                    error!("{:?}", transaction_error);
                }
                return Err(
                    format!("{} write transactions failed", transaction_errors.len()).into(),
                );
            }
        }
    }

    println!("Sending final message");
    if let Some(message) = final_message {
        if let Some(final_signers) = final_signers {
            trace!("Deploying program");
            let blockhash = rpc_client.get_latest_blockhash()?;

            let mut final_tx = Transaction::new_unsigned(message.clone());
            let mut signers = final_signers.to_vec();
            signers.push(payer_signer);
            final_tx.try_sign(&signers, blockhash)?;
            rpc_client
                .send_and_confirm_transaction_with_spinner_and_config(
                    &final_tx,
                    CommitmentConfig::confirmed(),
                    RpcSendTransactionConfig {
                        skip_preflight: true,
                        ..RpcSendTransactionConfig::default()
                    },
                )
                .map_err(|e| format!("Deploying program failed: {}", e))?;
        }
    }

    println!("Program deployed");
    Ok(())
}

fn add_program(program_name: &str) -> Account {
    let add_bpf = |program_file: PathBuf| {
        let data = read_file(&program_file);
        info!(
            "\"{}\" BPF program from {}{}",
            program_name,
            program_file.display(),
            std::fs::metadata(&program_file)
                .map(|metadata| {
                    metadata
                        .modified()
                        .map(|time| {
                            format!(
                                ", modified {}",
                                HumanTime::from(time).to_text_en(Accuracy::Precise, Tense::Past)
                            )
                        })
                        .ok()
                })
                .ok()
                .flatten()
                .unwrap_or_else(|| "".to_string())
        );

        return Account {
            lamports: Rent::default().minimum_balance(data.len()).min(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        };
    };

    let warn_invalid_program_name = || {
        let valid_program_names = default_shared_object_dirs()
            .iter()
            .filter_map(|dir| dir.read_dir().ok())
            .flat_map(|read_dir| {
                read_dir.filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if !path.is_file() {
                        return None;
                    }
                    match path.extension()?.to_str()? {
                        "so" => Some(path.file_stem()?.to_os_string()),
                        _ => None,
                    }
                })
            })
            .collect::<Vec<_>>();

        if valid_program_names.is_empty() {
            // This should be unreachable as `test-bpf` should guarantee at least one shared
            // object exists somewhere.
            warn!("No BPF shared objects found.");
            return;
        }

        warn!(
            "Possible bogus program name. Ensure the program name ({}) \
                matches one of the following recognizable program names:",
            program_name,
        );
        for name in valid_program_names {
            warn!(" - {}", name.to_str().unwrap());
        }
    };

    let program_file = find_file(&format!("{}.so", program_name));
    return match program_file {
        // If BPF is preferred (i.e., `test-bpf` is invoked) and a BPF shared object exists,
        // use that as the program data.
        Some(file) => add_bpf(file),

        // Invalid: `test-bpf` invocation with no matching BPF shared object.
        None => {
            warn_invalid_program_name();
            panic!("Program file data not available for {}", program_name);
        }
    };
}
