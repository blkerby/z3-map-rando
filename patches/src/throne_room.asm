lorom

; Allow pushing the throne room door with Zelda without the Lamp.
org $9AFC55
hook_throne_room_door_push:
    BRA return_throne_room_door_push

org $9AFC5B
return_throne_room_door_push:
