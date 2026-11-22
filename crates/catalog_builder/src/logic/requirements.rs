use std::collections::BTreeMap;

use logic_catalog::{
    Ammo, Equipment, FlagIndex, Follower, FollowerMask, Form, ItemIndex, OcarinaState, PrizeKind,
    Requirement, Resource, TechIndex, UniqueItem,
};

use super::{RoomIndices, z3_json_data};

pub struct Compiler {
    pub items: BTreeMap<String, ItemIndex>,
    pub item_use: BTreeMap<String, Vec<z3_json_data::Requirement>>,
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
        item: &str,
        cost_per_use: u32,
        num_uses: u32,
        room: &RoomIndices,
    ) -> Requirement {
        Requirement::make_and(vec![
            self.compile_named_requirement(item, room),
            Requirement::UseMagic {
                cost_per_use,
                num_uses,
            },
        ])
    }

    fn compile_item_use(
        &self,
        name: &str,
        ownership: Requirement,
        room: &RoomIndices,
    ) -> Requirement {
        match self.item_use.get(name) {
            Some(requires) => {
                Requirement::make_and(vec![ownership, self.compile_requirements(requires, room)])
            }
            None => ownership,
        }
    }

    pub(super) fn compile_named_requirement(&self, name: &str, room: &RoomIndices) -> Requirement {
        match name {
            // Great Fairy upgrades need no specific thrown item; the generator
            // places only upgrades whose prerequisite was already collected.
            "free" | "h_MothulaVulnerableToGoldSword" | "h_greatFairyFreeUpgrades" => {
                return Requirement::Free;
            }
            "never" => return Requirement::Never,
            "BigKey" => return Requirement::BigKey(room.dungeon_idx.unwrap()),
            "Bottle" => return Requirement::Bottles(1),
            "useABItems" => return Requirement::CanUseABItems,
            "useYItems" => return Requirement::CanUseYItems,
            "bunnyToSuperbunny" => {
                return Requirement::ChangeForm {
                    from: Form::Bunny,
                    to: Form::Superbunny,
                };
            }
            "bunnyToLonk" => {
                return Requirement::ChangeForm {
                    from: Form::Bunny,
                    to: Form::Lonk,
                };
            }
            "superbunnyToBunny" => {
                return Requirement::ChangeForm {
                    from: Form::Superbunny,
                    to: Form::Bunny,
                };
            }
            "superbunnyToLink" => {
                return Requirement::ChangeForm {
                    from: Form::Superbunny,
                    to: Form::Link,
                };
            }
            "lonkToLink" => {
                return Requirement::ChangeForm {
                    from: Form::Lonk,
                    to: Form::Link,
                };
            }
            "lonkToBunny" => {
                return Requirement::ChangeForm {
                    from: Form::Lonk,
                    to: Form::Bunny,
                };
            }
            "Ocarina" | "OcarinaInactive" => {
                return self.compile_item_use(
                    "OcarinaInactive",
                    Requirement::Ocarina(OcarinaState::Inactive),
                    room,
                );
            }
            "OcarinaActive" => {
                return self.compile_item_use(
                    name,
                    Requirement::Ocarina(OcarinaState::Active),
                    room,
                );
            }
            "BlueBoomerang" | "RedBoomerang" => {
                return self.compile_item_use(
                    name,
                    Requirement::Equipment {
                        equipment: Equipment::Boomerang,
                        minimum_level: if name == "BlueBoomerang" { 1 } else { 2 },
                    },
                    room,
                );
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
        let item = match name {
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
        let name = if name == "SilverArrowUpgrade" {
            "SilverArrows"
        } else {
            name
        };
        self.compile_item_use(name, Requirement::Item(item), room)
    }

    fn compile_requirement(
        &self,
        source: &z3_json_data::Requirement,
        room: &RoomIndices,
    ) -> Requirement {
        use z3_json_data::Operation as Op;
        let operation = match source {
            z3_json_data::Requirement::Named(name) => {
                return self.compile_named_requirement(name, room);
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
            Op::Sword(level) => self.compile_item_use(
                "L1Sword",
                Requirement::Equipment {
                    equipment: Equipment::Sword,
                    minimum_level: *level,
                },
                room,
            ),
            // The fairy upgrade accepts any shield at or above the given level.
            Op::Shield(level) | Op::ShieldExact(level) => self.compile_item_use(
                "BlueShield",
                Requirement::Equipment {
                    equipment: Equipment::Shield,
                    minimum_level: *level,
                },
                room,
            ),
            Op::Lift(level) | Op::Carry(level) => Requirement::make_and(vec![
                if matches!(operation, Op::Lift(_)) {
                    Requirement::CanUseABItems
                } else {
                    Requirement::IsLink
                },
                Requirement::Equipment {
                    equipment: Equipment::Glove,
                    minimum_level: *level,
                },
            ]),
            // We cannot rule out that the player was able to obtain a higher sword level.
            Op::SwordExact(_level) => Requirement::Never,
            Op::Arrows(count) | Op::SilverArrows(count) => {
                let mut requirements = vec![self.compile_named_requirement("Bow", room)];
                let kind = if matches!(operation, Op::SilverArrows(_)) {
                    requirements.push(self.compile_named_requirement("SilverArrows", room));
                    Ammo::SilverArrow
                } else {
                    Ammo::Arrow
                };
                requirements.push(Requirement::UseAmmo {
                    kind,
                    count: *count,
                });
                Requirement::make_and(requirements)
            }
            Op::Bombs(count) => Requirement::make_and(vec![
                Requirement::CanUseYItems,
                Requirement::LoseFollowers(FollowerMask::make([Follower::SuperBomb])),
                Requirement::UseAmmo {
                    kind: Ammo::Bomb,
                    count: *count,
                },
            ]),
            Op::MagicPowder(num_uses) => {
                self.compile_magic_requirement("MagicPowder", 8, *num_uses, room)
            }
            Op::FireRod(num_uses) => self.compile_magic_requirement("FireRod", 16, *num_uses, room),
            Op::IceRod(num_uses) => self.compile_magic_requirement("IceRod", 16, *num_uses, room),
            Op::Lamp(num_uses) => Requirement::make_and(vec![
                Requirement::CanUseYItems,
                self.compile_magic_requirement("Lamp", 4, *num_uses, room),
            ]),
            Op::RedCane(num_uses) => {
                self.compile_magic_requirement("CaneOfSomaria", 8, *num_uses, room)
            }
            Op::Cape(num_uses) => self.compile_magic_requirement("Cape", 1, *num_uses, room),
            Op::Rod(num_uses) => Requirement::make_and(vec![
                Requirement::make_or(vec![
                    self.compile_named_requirement("FireRod", room),
                    self.compile_named_requirement("IceRod", room),
                ]),
                Requirement::UseMagic {
                    cost_per_use: 16,
                    num_uses: *num_uses,
                },
            ]),
            Op::Bombos(num_uses) => self.compile_magic_requirement("Bombos", 32, *num_uses, room),
            Op::Ether(num_uses) => self.compile_magic_requirement("Ether", 32, *num_uses, room),
            Op::Quake(num_uses) => self.compile_magic_requirement("Quake", 32, *num_uses, room),
            Op::BlueCane(amount) => {
                let mut requirements = vec![
                    self.compile_named_requirement("CaneOfByrna", room),
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
