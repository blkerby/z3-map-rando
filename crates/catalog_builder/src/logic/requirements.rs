use std::collections::BTreeMap;

use logic_catalog::{
    Ammo, Equipment, FlagIndex, Follower, FollowerMask, ItemIndex, OcarinaState, PrizeKind,
    Requirement, Resource, TechIndex, UniqueItem,
};

use super::{RoomIndices, z3_json_data};

pub struct Compiler {
    pub items: BTreeMap<String, ItemIndex>,
    pub flags: BTreeMap<String, FlagIndex>,
    pub helpers: BTreeMap<String, z3_json_data::Helper>,
    pub techs: BTreeMap<String, (TechIndex, z3_json_data::Tech)>,
    pub damage: BTreeMap<String, [u32; 3]>,
}

impl Compiler {
    pub fn compile_requirements(
        &self,
        source: &[z3_json_data::Requirement],
        room: &RoomIndices,
    ) -> Requirement {
        let mut requirements = Vec::new();
        for requirement in source {
            requirements.push(self.compile_requirement(requirement, room));
        }
        Requirement::make_and(requirements)
    }

    fn compile_magic_requirement(
        &self,
        item: UniqueItem,
        cost_per_use: u32,
        num_uses: u32,
    ) -> Requirement {
        Requirement::make_and(vec![
            Requirement::Item(item),
            Requirement::UseMagic {
                cost_per_use,
                num_uses,
            },
        ])
    }

    fn compile_requirement(
        &self,
        source: &z3_json_data::Requirement,
        room: &RoomIndices,
    ) -> Requirement {
        use z3_json_data::Operation as Op;
        let operation = match source {
            z3_json_data::Requirement::Named(name) => {
                match name.as_str() {
                    "free" | "h_MothulaVulnerableToGoldSword" => return Requirement::Free,
                    "never" => return Requirement::Never,
                    "BigKey" => return Requirement::BigKey(room.dungeon_idx.unwrap()),
                    "Bottle" => return Requirement::Bottles(1),
                    "Ocarina" | "OcarinaInactive" => {
                        return Requirement::Ocarina(OcarinaState::Inactive);
                    }
                    "OcarinaActive" => return Requirement::Ocarina(OcarinaState::Active),
                    "BlueBoomerang" | "RedBoomerang" => {
                        return Requirement::make_and(vec![
                            Requirement::CanUseYItems,
                            Requirement::Equipment {
                                equipment: Equipment::Boomerang,
                                minimum_level: if name == "BlueBoomerang" { 1 } else { 2 },
                            },
                        ]);
                    }
                    _ => {}
                }
                if let Some(helper) = self.helpers.get(name) {
                    return self.compile_requirements(&helper.requires, room);
                }
                if let Some((tech_idx, tech)) = self.techs.get(name) {
                    return Requirement::make_and(vec![
                        Requirement::Tech(*tech_idx),
                        self.compile_requirements(&tech.tech_requires, room),
                        self.compile_requirements(&tech.other_requires, room),
                    ]);
                }
                let item = match name.as_str() {
                    "FireRod" => UniqueItem::FireRod,
                    "IceRod" => UniqueItem::IceRod,
                    "Hammer" => UniqueItem::Hammer,
                    "Hookshot" => UniqueItem::Hookshot,
                    "Bow" => UniqueItem::Bow,
                    "SilverArrows" | "SilverArrowUpgrade" => UniqueItem::SilverArrows,
                    "MagicPowder" => UniqueItem::MagicPowder,
                    "Bombos" => UniqueItem::Bombos,
                    "Ether" => UniqueItem::Ether,
                    "Quake" => UniqueItem::Quake,
                    "Lamp" => UniqueItem::Lamp,
                    "Shovel" => UniqueItem::Shovel,
                    "CaneOfSomaria" => UniqueItem::CaneOfSomaria,
                    "CaneOfByrna" => UniqueItem::CaneOfByrna,
                    "Cape" => UniqueItem::Cape,
                    "MagicMirror" => UniqueItem::MagicMirror,
                    "BookOfMudora" => UniqueItem::BookOfMudora,
                    "Flippers" => UniqueItem::Flippers,
                    "MoonPearl" => UniqueItem::MoonPearl,
                    "BugCatchingNet" => UniqueItem::BugCatchingNet,
                    "Mushroom" => UniqueItem::Mushroom,
                    "PegasusBoots" => UniqueItem::PegasusBoots,
                    "HalfMagic" => UniqueItem::HalfMagic,
                    "PendantOfCourage" => UniqueItem::PendantOfCourage,
                    "PendantOfWisdom" => UniqueItem::PendantOfWisdom,
                    "PendantOfPower" => UniqueItem::PendantOfPower,
                    "Crystal1" => UniqueItem::Crystal1,
                    "Crystal2" => UniqueItem::Crystal2,
                    "Crystal3" => UniqueItem::Crystal3,
                    "Crystal4" => UniqueItem::Crystal4,
                    "Crystal5" => UniqueItem::Crystal5,
                    "Crystal6" => UniqueItem::Crystal6,
                    "Crystal7" => UniqueItem::Crystal7,
                    "Triforce" => UniqueItem::Triforce,
                    _ => panic!("unknown requirement: {name}"),
                };
                return Requirement::Item(item);
            }
            z3_json_data::Requirement::Operation(operation) => operation,
        };
        match operation {
            Op::Notable(name) => Requirement::Notable(room.notables[name]),
            // Vanilla capacity purchases will be replaced by randomizer-specific behavior.
            Op::UpgradeCapacity(_) => Requirement::Never,
            Op::And(children) => self.compile_requirements(children, room),
            Op::Or(children) => {
                let mut branches = Vec::new();
                for child in children {
                    branches.push(self.compile_requirement(child, room));
                }
                Requirement::make_or(branches)
            }
            Op::Sword(level) => Requirement::make_and(vec![
                Requirement::CanInteract,
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: *level,
                },
            ]),
            // We intend to make the fairy upgrade to be applicable at any Shield level,
            // so the `shieldExact` requirement (only used as a condition for the fairy shield upgrade)
            // can be interpreted the same as a plain `shield` requirement.
            Op::Shield(level) | Op::ShieldExact(level) => Requirement::Equipment {
                equipment: Equipment::Shield,
                minimum_level: *level,
            },
            Op::Glove(level) => Requirement::make_and(vec![
                Requirement::CanInteract,
                Requirement::Equipment {
                    equipment: Equipment::Glove,
                    minimum_level: *level,
                },
            ]),
            // A `swordExact` level of 1 makes the Armos fight easier than level 2 sword.
            // We intend to leave this vanilla quirk unchanged, so we treat `swordExact` as
            // unsatisfiable, since we can't rule out that the player may have been
            // able to obtain a level 2 sword.
            Op::SwordExact(_level) => Requirement::Never,
            Op::Arrows(count) => Requirement::UseAmmo {
                kind: Ammo::Arrow,
                count: *count,
            },
            Op::SilverArrows(count) => Requirement::UseAmmo {
                kind: Ammo::SilverArrow,
                count: *count,
            },
            Op::Bombs(count) => Requirement::make_and(vec![
                Requirement::LoseFollowers(FollowerMask::make([Follower::SuperBomb])),
                Requirement::UseAmmo {
                    kind: Ammo::Bomb,
                    count: *count,
                },
            ]),
            Op::MagicPowder(num_uses) => {
                self.compile_magic_requirement(UniqueItem::MagicPowder, 8, *num_uses)
            }
            Op::FireRod(num_uses) => {
                self.compile_magic_requirement(UniqueItem::FireRod, 16, *num_uses)
            }
            Op::IceRod(num_uses) => {
                self.compile_magic_requirement(UniqueItem::IceRod, 16, *num_uses)
            }
            Op::Lamp(num_uses) => self.compile_magic_requirement(UniqueItem::Lamp, 4, *num_uses),
            Op::RedCane(num_uses) => {
                self.compile_magic_requirement(UniqueItem::CaneOfSomaria, 8, *num_uses)
            }
            Op::Cape(num_uses) => self.compile_magic_requirement(UniqueItem::Cape, 1, *num_uses),
            Op::Rod(num_uses) => Requirement::make_and(vec![
                Requirement::make_or(vec![
                    Requirement::Item(UniqueItem::FireRod),
                    Requirement::Item(UniqueItem::IceRod),
                ]),
                Requirement::UseMagic {
                    cost_per_use: 16,
                    num_uses: *num_uses,
                },
            ]),
            Op::Bombos(num_uses) => Requirement::make_and(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement(UniqueItem::Bombos, 32, *num_uses),
            ]),
            Op::Ether(num_uses) => Requirement::make_and(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement(UniqueItem::Ether, 32, *num_uses),
            ]),
            Op::Quake(num_uses) => Requirement::make_and(vec![
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: 1,
                },
                self.compile_magic_requirement(UniqueItem::Quake, 32, *num_uses),
            ]),
            Op::BlueCane(amount) => {
                let mut requirements = vec![
                    Requirement::Item(UniqueItem::CaneOfByrna),
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
                Requirement::make_and(requirements)
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
                None => Requirement::LoseFollowers(FollowerMask::make([
                    Follower::Zelda,
                    Follower::OldMan,
                    Follower::Blind,
                    Follower::Dwarf,
                    Follower::PurpleChest,
                    Follower::SuperBomb,
                ])),
            },
            Op::FollowerLost(followers) => {
                let mut lost = FollowerMask::default();
                for follower in followers {
                    lost.0 |= 1 << follower.get_follower().unwrap() as u8;
                }
                Requirement::LoseFollowers(lost)
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
                Requirement::make_and(requirements)
            }
            Op::ResourceMissingAtMost(resources) => {
                let mut requirements = Vec::new();
                for missing in resources {
                    let resource = missing.resource.get_resource();
                    let count = if resource == Resource::Health {
                        missing.count * 8
                    } else {
                        missing.count
                    };
                    requirements.push(Requirement::ResourceMissingAtMost { resource, count });
                }
                Requirement::make_and(requirements)
            }
        }
    }
}
