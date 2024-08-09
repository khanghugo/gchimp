use bsp::Bsp;
use dem::{
    bitslice_to_string,
    hldemo::{Demo, FrameData},
    nbit_str, parse_netmsg,
    types::EngineMessage,
    write_netmsg, Aux,
};

pub fn change_map(demo: &mut Demo, bsp: &Bsp, new_name: &str) {
    // new demo should request at least bsp_modelnum embedded models
    let bsp_modelnum = bsp.models.len();

    // add ".bsp" because people might be
    let new_name = if new_name.ends_with(".bsp") {
        new_name.to_owned()
    } else {
        new_name.to_owned() + ".bsp"
    };

    let mut header_new_name = vec![0u8; 260];
    header_new_name[..new_name.len()].copy_from_slice(new_name.as_bytes());

    demo.header.map_name = header_new_name.leak();

    let aux = Aux::new();

    for entry in &mut demo.directory.entries {
        for frame in &mut entry.frames {
            if let FrameData::NetMsg((_, data)) = &mut frame.data {
                let (_, mut netmsg) = parse_netmsg(data.msg, &aux).unwrap();
                for netmsg in &mut netmsg {
                    if let dem::types::NetMessage::EngineMessage(engine_message) = netmsg {
                        match engine_message.as_mut() {
                            EngineMessage::SvcServerInfo(server_info) => {
                                server_info.map_checksum = 0;
                                format!("{}\0", new_name)
                                    .as_bytes()
                                    .clone_into(&mut server_info.map_file_name);
                            }
                            EngineMessage::SvcResourceList(resource) => {
                                // even though it starts with 0, it is still safer to just parse the value
                                let mut map_entity_last = 0;
                                // insert idx is where it is in the array, this is independant from the actual entity index
                                let mut map_entity_last_insert_idx = 0;

                                for (res_index, res) in resource.resources.iter_mut().enumerate() {
                                    let res_name = bitslice_to_string(res.name.as_bitslice());

                                    if let Some(stripped) = res_name.strip_prefix("*") {
                                        map_entity_last_insert_idx = res_index;

                                        // have to trim becuase there's trailing nulls
                                        // but that is not enough
                                        map_entity_last = stripped
                                            .trim()
                                            .replace("\0", "")
                                            .parse::<usize>()
                                            .unwrap();
                                    }

                                    if res_name.starts_with("maps/") && res_name.ends_with(".bsp\0")
                                    {
                                        res.name = nbit_str!(format!("maps/{}\0", new_name));
                                    }
                                }

                                // if demo has more entity than map, then we have to delete
                                // the entity from the map
                                // that error is "Cannot continue without model *xxx, disconnecting"

                                // >= because if last entity is 9, we have 10 entities because entity 0
                                if map_entity_last >= bsp_modelnum {
                                    let remove_count = map_entity_last - bsp_modelnum + 1;

                                    for i in 0..remove_count {
                                        // lazily remove and that works
                                        // entity 1 is always player entity so we can count on that
                                        // the problem right now is that there will be this error
                                        // "Tried to link edict xxx without model"
                                        // but at least demo works
                                        resource.resources[map_entity_last_insert_idx - i].name =
                                            nbit_str!("*1");
                                    }
                                }

                                // otherwise, we pad entities so at least everything shows up
                                // UPDATE: this doesn't really work but hey
                                // the reason is that the entity won't just show up without somekind of delta update
                                // if bsp_modelnum > (map_entity_last + 1) {
                                //     (0..(bsp_modelnum - map_entity_last - 1)).for_each(
                                //         |new_entity_idx| {
                                //             println!("inserting entity {}", map_entity_last_idx + 1);
                                //             // fill with null terminators so it is consistent
                                //             let new_entity_name = format!(
                                //                 "*{}",
                                //                 map_entity_last + 1 + new_entity_idx
                                //             );
                                //             let new_entity_name = format!(
                                //                 "{}{}",
                                //                 new_entity_name,
                                //                 "\0".repeat(8 - new_entity_name.len())
                                //             );

                                //             let new_resource = Resource {
                                //                 type_: nbit_num!(2, 4),
                                //                 name: nbit_str!(new_entity_name),
                                //                 index: nbit_num!(map_entity_last_idx as usize + new_entity_idx + 1, 12),
                                //                 size: nbit_num!(0, 24),
                                //                 flags: nbit_num!(1, 3),
                                //                 md5_hash: None,
                                //                 has_extra_info: false,
                                //                 extra_info: None,
                                //             };

                                //             resource.resources.insert(
                                //                 map_entity_last_insert_idx + new_entity_idx + 1,
                                //                 new_resource,
                                //             );
                                //         },
                                //     );
                                // }
                            }
                            _ => (),
                        }
                    }
                }

                let bytes = write_netmsg(netmsg, &aux);
                data.msg = bytes.leak(); // hldemo does not own any data. Remember to free.
            }
        }
    }
}
