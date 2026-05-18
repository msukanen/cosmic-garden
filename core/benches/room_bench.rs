use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use cosmic_garden::room::{Room, environ::SPECIAL_ENVIRONMENT_DEFAULT};
use cosmic_garden::mob::core::Entity;
use cosmic_garden::traits::Tickable;
use cosmic_garden::thread::{librarian::librarian, signal::*};
use cosmic_garden::world::mock_world::get_operational_mock_world;
use cosmic_garden::identity::IdentityQuery;

// A helper to construct a hot, populated room for testing
async fn setup_bench_room(entity_count: usize, sigs: &SignalSenderChannels) -> Arc<RwLock<Room>> {
    let room = Room::new("bench-chamber", "Benchy", false).expect("Oh no!!!");
    {
        let mut room_lock = room.write().await;
        for i in 0..entity_count {
            let raw_id = format!("goblin:{i}");
            let mut goblin = Entity::new(&raw_id, &sigs).await.unwrap();
            #[cfg(feature = "stresstest")]{ log::trace!("{raw_id} ({})", goblin.tick_id()); }
            room_lock.add_entity(goblin.tick_id(), Arc::new(RwLock::new(goblin)));
        }
    }
    room
}

fn bench_room_tick(c: &mut Criterion) {
    let _ = env_logger::try_init();
    // Create a local, single-threaded runtime to isolate the 16-core chaos 
    // and measure the pure cost of the (a)synchronous logic math.
    let rt = tokio::runtime::Builder::new_current_thread()
        .worker_threads(4)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    let _rt_g = rt.enter();

    let mut group = c.benchmark_group("Room Throughput");
    let (w,sigs,(mut state, p),d) = rt.block_on(async {
        let (w,sigs,(state,p),d) = get_operational_mock_world().await;
        tokio::spawn( librarian((sigs.out.clone(), sigs.recv.librarian), w.clone()));
        (w,sigs.out,(state,p),d)
    });
    
    // Test scale scaling: 1 entity vs X entities in a single room
    for scale in &[1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
        let room = rt.block_on(setup_bench_room(*scale, &sigs));
        let mut curr_tick = 0;
        group.bench_with_input(
            criterion::BenchmarkId::new("tick_duration", scale),
            scale,
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let room_clone = room.clone();
                    curr_tick += 1;
                    async move {
                        let mut room_lock = room_clone.write().await;
                        // Execute the pure 100Hz marrow logic
                        room_lock.tick(
                            curr_tick, // current global tick count
                            room_clone.clone()
                        ).await;
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_room_tick);
criterion_main!(benches);
