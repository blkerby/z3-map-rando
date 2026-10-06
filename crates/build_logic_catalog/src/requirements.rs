use std::collections::BTreeMap;

use logic_catalog::{
    Ammo, Equipment, FlagIndex, Follower, ItemIndex, PrizeKind, Requirement, Resource, TechIndex,
};

use crate::{RoomIndices, source};

pub struct Compiler {
    pub items: BTreeMap<String, ItemIndex>,
    pub flags: BTreeMap<String, FlagIndex>,
    pub helpers: BTreeMap<String, source::Helper>,
    pub techs: BTreeMap<String, (TechIndex, source::Tech)>,
    pub damage: BTreeMap<String, [u32; 3]>,
}

/// Flatten conjunctions without changing execution order or resource actions.
pub fn compose_requirements(requirements: Vec<Requirement>) -> Requirement {
    let mut children = Vec::new();
    for requirement in requirements {
        match requirement {
            Requirement::Free => {}
            Requirement::And(nested) => children.extend(nested),
            other => children.push(other),
        }
    }
    match children.len() {
        0 => Requirement::Free,
        1 => children.pop().unwrap(),
        _ => Requirement::And(children.into_boxed_slice()),
    }
}

impl Compiler {
    pub fn compile_requirements(
        &self,
        source: &[source::Requirement],
        room: &RoomIndices,
    ) -> Requirement {
        let mut requirements = Vec::new();
        for requirement in source {
            requirements.push(self.compile_requirement(requirement, room));
        }
        compose_requirements(requirements)
    }

    fn compile_magic_requirement(
        &self,
        item: &str,
        cost_per_use: u32,
        num_uses: u32,
    ) -> Requirement {
        Requirement::And(Box::new([
            Requirement::Item(self.items[item]),
            Requirement::UseMagic {
                cost_per_use,
                num_uses,
            },
        ]))
    }

    fn compile_requirement(&self, source: &source::Requirement, room: &RoomIndices) -> Requirement {
        use source::Operation as Op;
        let operation = match source {
            source::Requirement::Named(name) => {
                match name.as_str() {
                    "free" | "h_MothulaVulnerableToGoldSword" => return Requirement::Free,
                    "never" => return Requirement::Never,
                    _ => {}
                }
                if let Some(helper) = self.helpers.get(name) {
                    return self.compile_requirements(&helper.requires, room);
                }
                if let Some((tech_idx, tech)) = self.techs.get(name) {
                    return compose_requirements(vec![
                        Requirement::Tech(*tech_idx),
                        self.compile_requirements(&tech.tech_requires, room),
                        self.compile_requirements(&tech.other_requires, room),
                    ]);
                }
                return Requirement::Item(self.items[name]);
            }
            source::Requirement::Operation(operation) => operation,
        };
        match operation {
            Op::And(children) => self.compile_requirements(children, room),
            Op::Or(children) => {
                let mut branches = Vec::new();
                for child in children {
                    branches.push(self.compile_requirement(child, room));
                }
                Requirement::Or(branches.into_boxed_slice())
            }
            Op::Sword(level) => Requirement::Equipment {
                equipment: Equipment::Sword,
                minimum_level: *level,
            },
            Op::Shield(level) => Requirement::Equipment {
                equipment: Equipment::Shield,
                minimum_level: *level,
            },
            Op::Glove(level) => Requirement::Equipment {
                equipment: Equipment::Glove,
                minimum_level: *level,
            },
            Op::SwordExact(_level) | Op::ShieldExact(_level) => Requirement::Never,
            Op::Arrows(count) => Requirement::UseAmmo {
                kind: Ammo::Arrow,
                count: *count,
            },
            Op::SilverArrows(count) => Requirement::UseAmmo {
                kind: Ammo::SilverArrow,
                count: *count,
            },
            Op::Bombs(count) => compose_requirements(vec![
                Requirement::LoseFollowers(Box::new([Follower::SuperBomb])),
                Requirement::UseAmmo {
                    kind: Ammo::Bomb,
                    count: *count,
                },
            ]),
            Op::MagicPowder(num_uses) => {
                self.compile_magic_requirement("MagicPowder", 8, *num_uses)
            }
            Op::FireRod(num_uses) => self.compile_magic_requirement("FireRod", 16, *num_uses),
            Op::IceRod(num_uses) => self.compile_magic_requirement("IceRod", 16, *num_uses),
            Op::Lamp(num_uses) => self.compile_magic_requirement("Lamp", 4, *num_uses),
            Op::RedCane(num_uses) => self.compile_magic_requirement("CaneOfSomaria", 8, *num_uses),
            Op::Cape(num_uses) => self.compile_magic_requirement("Cape", 1, *num_uses),
            Op::Rod(num_uses) => compose_requirements(vec![
                Requirement::Or(Box::new([
                    Requirement::Item(self.items["FireRod"]),
                    Requirement::Item(self.items["IceRod"]),
                ])),
                Requirement::UseMagic {
                    cost_per_use: 16,
                    num_uses: *num_uses,
                },
            ]),
            Op::Bombos(num_uses) => compose_requirements(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement("Bombos", 32, *num_uses),
            ]),
            Op::Ether(num_uses) => compose_requirements(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement("Ether", 32, *num_uses),
            ]),
            Op::Quake(num_uses) => compose_requirements(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement("Quake", 32, *num_uses),
            ]),
            Op::BlueCane(amount) => {
                let mut requirements = vec![
                    Requirement::Item(self.items["CaneOfByrna"]),
                    Requirement::UseMagic {
                        cost_per_use: 20,
                        num_uses: 1,
                    },
                ];
                let num_uses = (*amount - 20) / 4;
                if num_uses > 0 {
                    requirements.push(Requirement::UseMagic {
                        cost_per_use: 4,
                        num_uses,
                    });
                }
                compose_requirements(requirements)
            }
            Op::Damage(damage) => {
                let name = damage.attack.as_ref().unwrap_or(&damage.enemy);
                Requirement::Damage {
                    per_mail: self.damage[name],
                    hits: damage.count,
                }
            }
            Op::Refill(refill) => {
                let resource = refill.resource.get_resource();
                let limit = if resource == Resource::Health {
                    refill.limit * 8
                } else {
                    refill.limit
                };
                Requirement::Refill { resource, limit }
            }
            Op::CombatProficiency(minimum) => Requirement::Proficiency {
                tech_idx: self.techs["combatProficiency"].0,
                minimum: *minimum,
            },
            Op::BossProficiency(minimum) => Requirement::Proficiency {
                tech_idx: self.techs["bossProficiency"].0,
                minimum: *minimum,
            },
            Op::DarkProficiency(minimum) => Requirement::Proficiency {
                tech_idx: self.techs["darkProficiency"].0,
                minimum: *minimum,
            },
            Op::Pendants(minimum) => Requirement::PrizeCount {
                kind: PrizeKind::Pendant,
                minimum: *minimum,
            },
            Op::Crystals(minimum) => Requirement::PrizeCount {
                kind: PrizeKind::Crystal,
                minimum: *minimum,
            },
            Op::UnlockDoor(id) => Requirement::Door(room.doors[id]),
            Op::Pay(amount) => Requirement::Pay(*amount),
            Op::Flag(name) => Requirement::Flag(self.flags[name]),
            Op::NotFlag(_name) => Requirement::Never,
            Op::Follower(follower) => match follower.get_follower() {
                Some(follower) => Requirement::Follower(follower),
                None => Requirement::LoseFollowers(Box::new([
                    Follower::Zelda,
                    Follower::OldMan,
                    Follower::Blind,
                    Follower::Dwarf,
                    Follower::PurpleChest,
                    Follower::SuperBomb,
                ])),
            },
            Op::FollowerLost(followers) => {
                let mut lost = Vec::new();
                for follower in followers {
                    lost.push(follower.get_follower().unwrap());
                }
                Requirement::LoseFollowers(lost.into_boxed_slice())
            }
            Op::ObstaclesCleared(names) | Op::ObstaclesNotCleared(names) => {
                let mut requirements = Vec::new();
                for name in names {
                    let obstacle_idx = room.obstacles[name];
                    requirements.push(match operation {
                        Op::ObstaclesCleared(_) => Requirement::ObstacleCleared(obstacle_idx),
                        _ => Requirement::ObstacleNotCleared(obstacle_idx),
                    });
                }
                compose_requirements(requirements)
            }
            Op::ResourceMissingAtMost(resources) => {
                let mut requirements = Vec::new();
                for missing in resources {
                    requirements.push(Requirement::ResourceMissingAtMost {
                        resource: missing.resource.get_resource(),
                        count: missing.count,
                    });
                }
                compose_requirements(requirements)
            }
        }
    }
}
