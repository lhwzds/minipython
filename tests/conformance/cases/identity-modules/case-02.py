value = "非ASCII"
module_name = 'json'

module = __import__(module_name)
other = __import__(module_name)
log = CallLogger(); holder = Normal('holder', log, {value: module})
emit('identity', self=module is module, repeated=holder.value[value] is other, repeated_not=holder.value[value] is not other)
emit('module_operation', module=module_name, result=module.loads(module.dumps(value)))
