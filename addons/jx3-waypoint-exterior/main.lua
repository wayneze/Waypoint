-- 登录结束后写出当前角色已收集外观。只读白名单接口，不改收集状态。

local function quote(text)
	text = tostring(text or "")
	text = string.gsub(text, "\\", "\\\\")
	text = string.gsub(text, "\"", "\\\"")
	text = string.gsub(text, "\r", "")
	text = string.gsub(text, "\n", " ")
	return "\"" .. text .. "\""
end

local function call_name(fn, id)
	if not fn then
		return ""
	end
	local ok, value = pcall(fn, id)
	if ok and value and value ~= "" then
		return tostring(value)
	end
	return ""
end

local function player_name(player)
	if player and player.szName and player.szName ~= "" then
		return player.szName
	end
	return ""
end

local function player_server()
	local ok, name = pcall(GetUserServer)
	if ok and name and name ~= "" then
		return tostring(name)
	end
	return ""
end

local function file_token(text)
	text = tostring(text or "")
	if text == "" then
		return "unknown"
	end
	text = string.gsub(text, "[\\/:*?\"<>|#]", "_")
	return text
end

local function append_owned(lines, kind, ids, owned, name_of)
	if type(ids) ~= "table" then
		return 0
	end
	local count = 0
	for _, id in pairs(ids) do
		local number_id = tonumber(id)
		if number_id and (not owned or owned(number_id)) then
			table.insert(lines, string.format(
				"    {kind=%s,id=%d,name=%s},",
				quote(kind),
				number_id,
				quote(call_name(name_of, number_id))
			))
			count = count + 1
		end
	end
	return count
end

local function hair_owned(player, id, name)
	if not player.IsHaveHair then
		return false
	end
	if id then
		local ok, have = pcall(player.IsHaveHair, player, id)
		if ok and have then
			return true
		end
	end
	if name ~= "" then
		local ok, have = pcall(player.IsHaveHair, player, name)
		return ok and have
	end
	return false
end

local function hair_label(id, name)
	if name ~= "" then
		return name
	end
	if not id then
		return ""
	end
	local from_shop = call_name(GetHairShop, id)
	if from_shop ~= "" then
		return from_shop
	end
	return "发型#" .. tostring(id)
end

local function append_hair(lines, player)
	if not player.GetAllHair then
		return 0
	end
	local hairs = player.GetAllHair()
	if type(hairs) ~= "table" then
		return 0
	end
	local count = 0
	local seen = {}
	for key, value in pairs(hairs) do
		local id = tonumber(key) or tonumber(value)
		local name = ""
		if type(value) == "string" and value ~= "" and not tonumber(value) then
			name = value
		elseif type(key) == "string" and not tonumber(key) then
			name = key
		end
		if hair_owned(player, id, name) then
			local label = hair_label(id, name)
			local dedupe = label ~= "" and label or ("id:" .. tostring(id or key))
			if not seen[dedupe] then
				seen[dedupe] = true
				table.insert(lines, string.format(
					"    {kind=%s,id=%d,name=%s},",
					quote("hair"),
					id or 0,
					quote(label)
				))
				count = count + 1
			end
		end
	end
	return count
end

function WaypointExterior_Write()
	local player = GetClientPlayer()
	if not player then
		return
	end
	local role = player_name(player)
	local server = player_server()
	local lines = {
		"return {",
		"  role=" .. quote(role) .. ",",
		"  server=" .. quote(server) .. ",",
		"  items={",
	}
	local total = 0
	local all_body = player.GetAllExterior and player.GetAllExterior() or nil
	total = total + append_owned(lines, "body", all_body, function(id)
		return player.IsExteriorCollected and player.IsExteriorCollected(id)
	end, Exterior_GetExteriorName)
	local weapons = player.GetAllWeaponExterior and player.GetAllWeaponExterior() or nil
	total = total + append_owned(lines, "weapon", weapons, function(id)
		return player.IsWeaponExteriorCollected and player.IsWeaponExteriorCollected(id)
	end, function(id)
		local name = call_name(player.GetWeaponExteriorSet, id)
		if name ~= "" then
			return name
		end
		return call_name(Exterior_GetExteriorName, id)
	end)
	total = total + append_hair(lines, player)
	table.insert(lines, "  },")
	table.insert(lines, "}")
	local file_name = "exterior_" .. file_token(server) .. "_" .. file_token(role)
	SaveLUAData("interface/Waypoint#DATA/" .. file_name, table.concat(lines, "\n"))
	OutputMessage("MSG_SYS", string.format("Waypoint 外观快照已写出 %d 件\n", total))
end

RegisterEvent("LOADING_END", function()
	DelayCall(1500, function()
		local ok, err = pcall(WaypointExterior_Write)
		if not ok then
			OutputMessage("MSG_SYS", "Waypoint 外观快照失败 " .. tostring(err) .. "\n")
		end
	end)
end)
