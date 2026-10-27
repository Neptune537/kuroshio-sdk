use kuroshio_mesh::{KuroshioTeam, MemberRole, SquadMember};
use lumeria_app_sdk::{AppCapability, AppManifest, LumeriaApp, LumeriaHostCommand, LumeriaInputEvent};
use nautilus_world::{AnchoredSpatialOverlay, Wgs84GeoCoord};

pub struct TeamApp {
    pub team: KuroshioTeam,
    pub arena_overlay: AnchoredSpatialOverlay,
}

impl TeamApp {
    pub fn new() -> Self {
        let tokyo_tower = Wgs84GeoCoord::new(35.658580, 139.745433, 20.0);
        let mut team = KuroshioTeam::new(
            "team_seraphim",
            "Seraphim Tactical Wing",
            "SERAPH",
            "did:atlantis:player1_founder",
            "Vance",
        );
        team.set_headquarters(tokyo_tower);

        let arena = AnchoredSpatialOverlay::new(
            "overlay_tokyo_arena",
            "Tokyo Tower Tactical Zone",
            tokyo_tower,
            1500.0,
        );

        Self {
            team,
            arena_overlay: arena,
        }
    }
}

impl LumeriaApp for TeamApp {
    fn on_init(&mut self) -> Result<(), String> {
        println!("Team {} [tag: {}] initialized on Nautilus World!", self.team.name, self.team.tag);
        Ok(())
    }

    fn on_update(&mut self, _delta_seconds: f32) -> Vec<LumeriaHostCommand> {
        vec![]
    }

    fn on_input(&mut self, event: LumeriaInputEvent) -> Vec<LumeriaHostCommand> {
        match event {
            LumeriaInputEvent::ProximityTrigger { totem_id, distance_meters } => {
                vec![LumeriaHostCommand::ShowNotification {
                    title: format!("Team HQ Proximity: {}", self.team.tag),
                    message: format!("Approached totem {} within {:.1}m", totem_id, distance_meters),
                }]
            }
            _ => vec![],
        }
    }
}

fn main() {
    let mut app = TeamApp::new();
    let _ = app.on_init();
    let frame = app.arena_overlay.tangent_frame();
    println!("Arena anchor ECEF: {:?}", frame.origin_ecef);

    let manifest = AppManifest::new("com.seraph.tactical", "Seraphim Tactical", "0.1.0", "did:atlantis:publisher1")
        .with_capability(AppCapability::LocationCoarse)
        .with_capability(AppCapability::UiOverlay);

    println!("App Manifest validated: {:?}", manifest.validate_permissions());
}
