#pragma once

#include "core/io/missing_resource.h"
#include <core/object/script_language.h>
#include <core/templates/rb_set.h>

#include <bytecode/bytecode_base.h>

class FakeGDScript : public Script {
	GDCLASS(FakeGDScript, Script);
	bool tool = false;
	bool valid = false;
	bool reloading = false;
	bool is_binary = false;
	String original_class = "GDScript";

	using GlobalToken = GDScriptDecomp::GlobalToken;
	// Ref<GDScriptNativeClass> native;
	Ref<FakeGDScript> base;
	FakeGDScript *_base = nullptr; //fast pointer access
	FakeGDScript *_owner = nullptr; //for subclasses

	// Members are just indices to the instantiated script.
	// HashMap<StringName, MemberInfo> member_indices; // Includes member info of all base GDScript classes.
	HashSet<StringName> members; // Only members of the current class.

	// Only static variables of the current class.
	// HashMap<StringName, MemberInfo> static_variables_indices;
	Vector<Variant> static_variables; // Static variable values.

	String source;
	int override_bytecode_revision = 0;
	// Vector<uint8_t> binary_tokens;
	GDScriptDecomp::ScriptState script_state;
	String script_path;
	bool path_valid = false; // False if using default path.
	StringName base_type; // `extends`.
	StringName local_name; // Inner class identifier or `class_name`.
	StringName global_name; // `class_name`.

	//	HashMap<StringName, Variant> constants;
	// String fully_qualified_name;
	// String simplified_icon_path;
	Ref<GDScriptDecomp> decomp;
	HashMap<StringName, Pair<int, int>> subclasses;
	Vector<uint8_t> binary_buffer;

	String error_message;

	Error parse_script();

protected:
	bool _get(const StringName &p_name, Variant &r_ret) const;
	bool _set(const StringName &p_name, const Variant &p_value);
	void _get_property_list(List<PropertyInfo> *p_properties) const;
	Variant callp(const StringName &p_method, const Variant **p_args, int p_argcount,
			Callable::CallError &r_error) override;

	static void _bind_methods();

public:
	Error _reload_from_file();
	virtual void reload_from_file() override;

	virtual bool can_instantiate() const override;

	Ref<Script> get_base_script() const override; //for script inheritance
	virtual StringName get_global_name() const override;
	virtual bool inherits_script(const Ref<Script> &p_script) const override;

	virtual StringName get_instance_base_type() const override;
	// this may not work in all scripts, will return empty if so
	virtual ScriptInstance *instance_create(Object *p_this) override;
	virtual PlaceHolderScriptInstance *placeholder_instance_create(Object *p_this) override;
	virtual bool instance_has(const Object *p_this) const override;

	virtual bool has_source_code() const override;
	virtual String get_source_code() const override;
	virtual void set_source_code(const String &p_code) override;
	virtual Error reload(bool p_keep_state = false) override;

#ifdef TOOLS_ENABLED
	virtual StringName get_doc_class_name() const override;
	virtual Vector<DocData::ClassDoc> get_documentation() const override;
	virtual String get_class_icon_path() const override;
	virtual PropertyInfo get_class_category() const override;
#endif // TOOLS_ENABLED

	// TODO: In the next compat breakage rename to `*_script_*` to disambiguate from `Object::has_method()`.
	virtual bool has_method(const StringName &p_method) const override;
	virtual bool has_static_method(const StringName &p_method) const override;

	virtual int get_script_method_argument_count(const StringName &p_method, bool *r_is_valid = nullptr) const override;

	virtual MethodInfo get_method_info(const StringName &p_method) const override;

	virtual bool is_tool() const override;
	virtual bool is_valid() const override;
	virtual bool is_abstract() const override;

	virtual ScriptLanguage *get_language() const override;

	virtual bool has_script_signal(const StringName &p_signal) const override;
	virtual void get_script_signal_list(List<MethodInfo> *r_signals) const override;

	virtual bool get_property_default_value(const StringName &p_property, Variant &r_value) const override;

	virtual void update_exports() override;
	//editor tool
	virtual void get_script_method_list(List<MethodInfo> *p_list) const override;
	virtual void get_script_property_list(List<PropertyInfo> *p_list) const override;

	virtual int get_member_line(const StringName &p_member) const override;

	virtual void get_constants(HashMap<StringName, Variant> *p_constants) override;
	virtual void get_members(HashSet<StringName> *p_members) override;

	virtual bool is_placeholder_fallback_enabled() const override;

	virtual const Variant get_rpc_config() const override;
	virtual String get_save_class() const override { return original_class; }

	String get_script_path() const;
	Error load_source_code(const String &p_path);

	String get_error_message() const;

	void set_override_bytecode_revision(int p_revision);
	int get_override_bytecode_revision() const;
};

class FakeEmbeddedScript : public Script {
	GDCLASS(FakeEmbeddedScript, Script);
	String original_class;
	HashMap<StringName, Variant> properties;

protected:
	bool _get(const StringName &p_name, Variant &r_ret) const;
	bool _set(const StringName &p_name, const Variant &p_value);
	void _get_property_list(List<PropertyInfo> *p_list) const;

public:
	virtual void reload_from_file() override {}

	virtual bool can_instantiate() const override { return false; }

	virtual Ref<Script> get_base_script() const override { return nullptr; } //for script inheritance
	virtual StringName get_global_name() const override;
	virtual bool inherits_script(const Ref<Script> &p_script) const override;

	virtual StringName get_instance_base_type() const override;
	virtual ScriptInstance *instance_create(Object *p_this) override { return nullptr; }
	virtual PlaceHolderScriptInstance *placeholder_instance_create(Object *p_this) override { return nullptr; }
	virtual bool instance_has(const Object *p_this) const override { return false; }

	virtual bool has_source_code() const override;
	virtual String get_source_code() const override;
	virtual void set_source_code(const String &p_code) override;
	virtual Error reload(bool p_keep_state = false) override { return OK; }

#ifdef TOOLS_ENABLED
	virtual StringName get_doc_class_name() const override { return {}; }
	virtual Vector<DocData::ClassDoc> get_documentation() const override { return {}; }
	virtual String get_class_icon_path() const override { return {}; }
	virtual PropertyInfo get_class_category() const override { return {}; }
#endif // TOOLS_ENABLED

	// TODO: In the next compat breakage rename to `*_script_*` to disambiguate from `Object::has_method()`.
	virtual bool has_method(const StringName &p_method) const override { return false; }
	virtual bool has_static_method(const StringName &p_method) const override { return false; }

	virtual int get_script_method_argument_count(const StringName &p_method, bool *r_is_valid = nullptr) const override { return 0; }

	virtual MethodInfo get_method_info(const StringName &p_method) const override { return {}; }

	virtual bool is_tool() const override { return false; }
	virtual bool is_valid() const override { return true; }
	virtual bool is_abstract() const override { return false; }

	virtual ScriptLanguage *get_language() const override { return nullptr; }

	virtual bool has_script_signal(const StringName &p_signal) const override { return false; }
	virtual void get_script_signal_list(List<MethodInfo> *r_signals) const override {}

	virtual bool get_property_default_value(const StringName &p_property, Variant &r_value) const override { return false; }

	virtual void update_exports() override {} //editor tool
	virtual void get_script_method_list(List<MethodInfo> *p_list) const override {}
	virtual void get_script_property_list(List<PropertyInfo> *p_list) const override {}

	virtual int get_member_line(const StringName &p_member) const override { return -1; }

	virtual void get_constants(HashMap<StringName, Variant> *p_constants) override {}
	virtual void get_members(HashSet<StringName> *p_members) override {}

	virtual bool is_placeholder_fallback_enabled() const override { return false; }

	virtual const Variant get_rpc_config() const override { return {}; }
	virtual String get_save_class() const override { return original_class; }

	void set_original_class(const String &p_class);
	String get_original_class() const;
};
